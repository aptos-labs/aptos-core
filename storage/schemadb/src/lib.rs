// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This library implements a schematized DB on top of [RocksDB](https://rocksdb.org/). It makes
//! sure all data passed in and out are structured according to predefined schemas and prevents
//! access to raw keys and values. This library also enforces a set of specific DB options,
//! like custom comparators and schema-to-column-family mapping.
//!
//! It requires that different kinds of key-value pairs be stored in separate column
//! families.  To use this library to store a kind of key-value pairs, the user needs to use the
//! [`define_schema!`] macro to define the schema name, the types of key and value, and name of the
//! column family.

mod metrics;
#[macro_use]
pub mod schema;
pub mod iterator;

use crate::{
    metrics::{
        APTOS_SCHEMADB_BATCH_COMMIT_BYTES, APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS,
        APTOS_SCHEMADB_DELETES_SAMPLED, APTOS_SCHEMADB_GET_BYTES,
        APTOS_SCHEMADB_GET_LATENCY_SECONDS, APTOS_SCHEMADB_ITER_BYTES,
        APTOS_SCHEMADB_ITER_LATENCY_SECONDS, APTOS_SCHEMADB_PUT_BYTES_SAMPLED,
        APTOS_SCHEMADB_SEEK_LATENCY_SECONDS, TIMER,
    },
    schema::{KeyCodec, Schema, SeekKeyCodec, ValueCodec},
};
use anyhow::format_err;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{AptosDbError, Result as DbResult};
use iterator::{ScanDirection, SchemaIterator};
use rand::Rng;
use rocksdb::ErrorKind;
/// Type alias to `rocksdb::ReadOptions`. See [`rocksdb doc`](https://github.com/pingcap/rust-rocksdb/blob/master/src/rocksdb_options.rs)
pub use rocksdb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, DBCompressionType, Options, ReadOptions,
    SliceTransform, DEFAULT_COLUMN_FAMILY_NAME,
};
use std::{
    collections::{HashMap, HashSet},
    iter::Iterator,
    path::Path,
};

pub type ColumnFamilyName = &'static str;

#[derive(Debug)]
enum WriteOp {
    Value { key: Vec<u8>, value: Vec<u8> },
    Deletion { key: Vec<u8> },
}

/// `SchemaBatch` holds a collection of updates that can be applied to a DB atomically. The updates
/// will be applied in the order in which they are added to the `SchemaBatch`.
#[derive(Debug, Default)]
pub struct SchemaBatch {
    rows: HashMap<ColumnFamilyName, Vec<WriteOp>>,
}

impl SchemaBatch {
    /// Creates an empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an insert/update operation to the batch.
    pub fn put<S: Schema>(
        &mut self,
        key: &S::Key,
        value: &S::Value,
    ) -> aptos_storage_interface::Result<()> {
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;
        let value = <S::Value as ValueCodec<S>>::encode_value(value)?;
        self.rows
            .entry(S::COLUMN_FAMILY_NAME)
            .or_default()
            .push(WriteOp::Value { key, value });

        Ok(())
    }

    /// Adds a delete operation to the batch.
    pub fn delete<S: Schema>(&mut self, key: &S::Key) -> DbResult<()> {
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;
        self.rows
            .entry(S::COLUMN_FAMILY_NAME)
            .or_default()
            .push(WriteOp::Deletion { key });

        Ok(())
    }
}

#[derive(Debug)]
enum OpenMode<'a> {
    ReadWrite,
    ReadOnly,
    Secondary(&'a Path),
}

/// This DB is a schematized RocksDB wrapper where all data passed in and out are typed according to
/// [`Schema`]s.
#[derive(Debug)]
pub struct DB {
    name: String, // for logging
    inner: rocksdb::DB,
}

impl DB {
    pub fn open(
        path: impl AsRef<Path>,
        name: &str,
        column_families: Vec<ColumnFamilyName>,
        db_opts: &Options,
    ) -> DbResult<Self> {
        let db = DB::open_cf(
            db_opts,
            path,
            name,
            column_families
                .iter()
                .map(|cf_name| {
                    let mut cf_opts = Options::default();
                    cf_opts.set_compression_type(DBCompressionType::Lz4);
                    ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts)
                })
                .collect(),
        )?;
        Ok(db)
    }

    pub fn open_cf(
        db_opts: &Options,
        path: impl AsRef<Path>,
        name: &str,
        cfds: Vec<ColumnFamilyDescriptor>,
    ) -> DbResult<DB> {
        Self::open_cf_impl(db_opts, path, name, cfds, OpenMode::ReadWrite)
    }

    /// Open db in readonly mode
    /// Note that this still assumes there's only one process that opens the same DB.
    /// See `open_as_secondary`
    pub fn open_cf_readonly(
        opts: &Options,
        path: impl AsRef<Path>,
        name: &str,
        cfds: Vec<ColumnFamilyDescriptor>,
    ) -> DbResult<DB> {
        Self::open_cf_impl(opts, path, name, cfds, OpenMode::ReadOnly)
    }

    pub fn open_cf_as_secondary<P: AsRef<Path>>(
        opts: &Options,
        primary_path: P,
        secondary_path: P,
        name: &str,
        cfds: Vec<ColumnFamilyDescriptor>,
    ) -> DbResult<DB> {
        Self::open_cf_impl(
            opts,
            primary_path,
            name,
            cfds,
            OpenMode::Secondary(secondary_path.as_ref()),
        )
    }

    fn open_cf_impl(
        db_opts: &Options,
        path: impl AsRef<Path>,
        name: &str,
        cfds: Vec<ColumnFamilyDescriptor>,
        open_mode: OpenMode,
    ) -> DbResult<DB> {
        // ignore error, since it'll fail to list cfs on the first open
        let existing_cfs = rocksdb::DB::list_cf(db_opts, path.de_unc()).unwrap_or_default();

        let unrecognized_cfds = existing_cfs
            .iter()
            .map(AsRef::as_ref)
            .collect::<HashSet<&str>>()
            .difference(&cfds.iter().map(|cfd| cfd.name()).collect())
            .map(|cf| {
                warn!("Unrecognized CF: {}", cf);

                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(DBCompressionType::Lz4);
                ColumnFamilyDescriptor::new(cf.to_string(), cf_opts)
            })
            .collect::<Vec<_>>();
        let all_cfds = cfds.into_iter().chain(unrecognized_cfds);

        let inner = {
            use rocksdb::DB;
            use OpenMode::*;

            match open_mode {
                ReadWrite => DB::open_cf_descriptors(db_opts, path.de_unc(), all_cfds),
                ReadOnly => {
                    DB::open_cf_descriptors_read_only(
                        db_opts,
                        path.de_unc(),
                        all_cfds,
                        false, /* error_if_log_file_exist */
                    )
                },
                Secondary(secondary_path) => DB::open_cf_descriptors_as_secondary(
                    db_opts,
                    path.de_unc(),
                    secondary_path,
                    all_cfds,
                ),
            }
        }
        .into_db_res()?;

        Ok(Self::log_construct(name, open_mode, inner))
    }

    fn log_construct(name: &str, open_mode: OpenMode, inner: rocksdb::DB) -> DB {
        info!(
            rocksdb_name = name,
            open_mode = ?open_mode,
            "Opened RocksDB."
        );
        DB {
            name: name.to_string(),
            inner,
        }
    }

    /// Reads single record by key.
    pub fn get<S: Schema>(&self, schema_key: &S::Key) -> DbResult<Option<S::Value>> {
        let _timer = APTOS_SCHEMADB_GET_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .start_timer();

        let k = <S::Key as KeyCodec<S>>::encode_key(schema_key)?;
        let cf_handle = self.get_cf_handle(S::COLUMN_FAMILY_NAME)?;

        let result = self.inner.get_cf(cf_handle, k).into_db_res()?;
        APTOS_SCHEMADB_GET_BYTES
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .observe(result.as_ref().map_or(0.0, |v| v.len() as f64));

        result
            .map(|raw_value| <S::Value as ValueCodec<S>>::decode_value(&raw_value))
            .transpose()
            .map_err(Into::into)
    }

    /// Writes single record.
    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> DbResult<()> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        let mut batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.write_schemas(batch)
    }

    /// Deletes a single record.
    pub fn delete<S: Schema>(&self, key: &S::Key) -> DbResult<()> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        let mut batch = SchemaBatch::new();
        batch.delete::<S>(key)?;
        self.write_schemas(batch)
    }

    fn iter_with_direction<S: Schema>(
        &self,
        opts: ReadOptions,
        direction: ScanDirection,
    ) -> DbResult<SchemaIterator<S>> {
        let cf_handle = self.get_cf_handle(S::COLUMN_FAMILY_NAME)?;
        Ok(SchemaIterator::new(
            self.inner.raw_iterator_cf_opt(cf_handle, opts),
            direction,
        ))
    }

    /// Returns a forward [`SchemaIterator`] on a certain schema.
    pub fn iter<S: Schema>(&self) -> DbResult<SchemaIterator<S>> {
        self.iter_with_opts(ReadOptions::default())
    }

    /// Returns a forward [`SchemaIterator`] on a certain schema, with non-default ReadOptions
    pub fn iter_with_opts<S: Schema>(&self, opts: ReadOptions) -> DbResult<SchemaIterator<S>> {
        self.iter_with_direction::<S>(opts, ScanDirection::Forward)
    }

    /// Returns a backward [`SchemaIterator`] on a certain schema.
    pub fn rev_iter<S: Schema>(&self) -> DbResult<SchemaIterator<S>> {
        self.rev_iter_with_opts(ReadOptions::default())
    }

    /// Returns a backward [`SchemaIterator`] on a certain schema, with non-default ReadOptions
    pub fn rev_iter_with_opts<S: Schema>(&self, opts: ReadOptions) -> DbResult<SchemaIterator<S>> {
        self.iter_with_direction::<S>(opts, ScanDirection::Backward)
    }

    /// Writes a group of records wrapped in a [`SchemaBatch`].
    pub fn write_schemas(&self, batch: SchemaBatch) -> DbResult<()> {
        self.write_in_one_db_batch(vec![batch])
    }

    pub fn write_in_one_db_batch(&self, batches: Vec<SchemaBatch>) -> DbResult<()> {
        // Function to determine if the counter should be sampled based on a sampling percentage
        fn should_sample(sampling_percentage: usize) -> bool {
            // Generate a random number between 0 and 100
            let random_value = rand::thread_rng().gen_range(0, 100);

            // Sample the counter if the random value is less than the sampling percentage
            random_value <= sampling_percentage
        }

        let _timer = APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS
            .with_label_values(&[&self.name])
            .start_timer();
        let sampling_rate_pct = 1;
        let sampled_kv_bytes = should_sample(sampling_rate_pct);

        let db_batch = {
            let _timer = TIMER.timer_with(&["convert_to_db_batch", &self.name]);

            let mut ret = rocksdb::WriteBatch::default();
            for batch in &batches {
                for (cf_name, rows) in batch.rows.iter() {
                    let cf_handle = self.get_cf_handle(cf_name)?;
                    for write_op in rows {
                        match write_op {
                            WriteOp::Value { key, value } => ret.put_cf(cf_handle, key, value),
                            WriteOp::Deletion { key } => ret.delete_cf(cf_handle, key),
                        }
                    }
                }
            }
            ret
        };
        let serialized_size = db_batch.size_in_bytes();

        self.inner
            .write_opt(db_batch, &default_write_options())
            .into_db_res()?;

        // Bump counters only after DB write succeeds.
        if sampled_kv_bytes {
            for batch in batches {
                for (cf_name, rows) in batch.rows.iter() {
                    for write_op in rows {
                        match write_op {
                            WriteOp::Value { key, value } => {
                                APTOS_SCHEMADB_PUT_BYTES_SAMPLED
                                    .with_label_values(&[cf_name])
                                    .observe((key.len() + value.len()) as f64);
                            },
                            WriteOp::Deletion { key: _ } => {
                                APTOS_SCHEMADB_DELETES_SAMPLED
                                    .with_label_values(&[cf_name])
                                    .inc();
                            },
                        }
                    }
                }
            }
        }

        APTOS_SCHEMADB_BATCH_COMMIT_BYTES
            .with_label_values(&[&self.name])
            .observe(serialized_size as f64);

        Ok(())
    }

    fn get_cf_handle(&self, cf_name: &str) -> DbResult<&rocksdb::ColumnFamily> {
        self.inner
            .cf_handle(cf_name)
            .ok_or_else(|| {
                format_err!(
                    "DB::cf_handle not found for column family name: {}",
                    cf_name
                )
            })
            .map_err(Into::into)
    }

    /// Flushes memtable data. This is only used for testing `get_approximate_sizes_cf` in unit
    /// tests.
    pub fn flush_cf(&self, cf_name: &str) -> DbResult<()> {
        self.inner
            .flush_cf(self.get_cf_handle(cf_name)?)
            .into_db_res()
    }

    pub fn get_property(&self, cf_name: &str, property_name: &str) -> DbResult<u64> {
        self.inner
            .property_int_value_cf(self.get_cf_handle(cf_name)?, property_name)
            .into_db_res()?
            .ok_or_else(|| {
                aptos_storage_interface::AptosDbError::Other(
                    format!(
                        "Unable to get property \"{}\" of  column family \"{}\".",
                        property_name, cf_name,
                    )
                    .to_string(),
                )
            })
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> DbResult<()> {
        rocksdb::checkpoint::Checkpoint::new(&self.inner)
            .into_db_res()?
            .create_checkpoint(path)
            .into_db_res()?;
        Ok(())
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        info!(rocksdb_name = self.name, "Dropped RocksDB.");
    }
}

/// For now we always use synchronous writes. This makes sure that once the operation returns
/// `Ok(())` the data is persisted even if the machine crashes. In the future we might consider
/// selectively turning this off for some non-critical writes to improve performance.
fn default_write_options() -> rocksdb::WriteOptions {
    let mut opts = rocksdb::WriteOptions::default();
    opts.set_sync(true);
    opts
}

trait DeUnc: AsRef<Path> {
    fn de_unc(&self) -> &Path {
        // `dunce` is needed to "de-UNC" because rocksdb doesn't take Windows UNC paths like `\\?\C:\`
        dunce::simplified(self.as_ref())
    }
}

impl<T> DeUnc for T where T: AsRef<Path> {}

fn to_db_err(rocksdb_err: rocksdb::Error) -> AptosDbError {
    match rocksdb_err.kind() {
        ErrorKind::Incomplete => AptosDbError::RocksDbIncompleteResult(rocksdb_err.to_string()),
        ErrorKind::NotFound
        | ErrorKind::Corruption
        | ErrorKind::NotSupported
        | ErrorKind::InvalidArgument
        | ErrorKind::IOError
        | ErrorKind::MergeInProgress
        | ErrorKind::ShutdownInProgress
        | ErrorKind::TimedOut
        | ErrorKind::Aborted
        | ErrorKind::Busy
        | ErrorKind::Expired
        | ErrorKind::TryAgain
        | ErrorKind::CompactionTooLarge
        | ErrorKind::ColumnFamilyDropped
        | ErrorKind::Unknown => AptosDbError::OtherRocksDbError(rocksdb_err.to_string()),
    }
}

trait IntoDbResult<T> {
    fn into_db_res(self) -> DbResult<T>;
}

impl<T> IntoDbResult<T> for Result<T, rocksdb::Error> {
    fn into_db_res(self) -> DbResult<T> {
        self.map_err(to_db_err)
    }
}
