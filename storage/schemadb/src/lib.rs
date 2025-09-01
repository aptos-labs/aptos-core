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
pub mod batch;
pub mod iterator;

use crate::{
    metrics::{
        APTOS_SCHEMADB_BATCH_COMMIT_BYTES, APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS,
        APTOS_SCHEMADB_GET_BYTES, APTOS_SCHEMADB_GET_LATENCY_SECONDS, APTOS_SCHEMADB_ITER_BYTES,
        APTOS_SCHEMADB_ITER_LATENCY_SECONDS, APTOS_SCHEMADB_SEEK_LATENCY_SECONDS,
    },
    schema::{KeyCodec, Schema, SeekKeyCodec, ValueCodec},
};
use anyhow::format_err;
use aptos_logger::prelude::*;
use aptos_storage_interface::{AptosDbError, Result as DbResult};
use batch::{IntoRawBatch, NativeBatch, WriteBatch};
use iterator::{ScanDirection, SchemaIterator};
/// Type alias to `rocksdb::ReadOptions`. See [`rocksdb doc`](https://github.com/pingcap/rust-rocksdb/blob/master/src/rocksdb_options.rs)
pub use rocksdb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, DBCompressionType, Options, ReadOptions,
    SliceTransform, DEFAULT_COLUMN_FAMILY_NAME,
};
use rocksdb::{ErrorKind, WriteOptions};
use std::{collections::HashSet, fmt::Debug, iter::Iterator, path::Path};

pub type ColumnFamilyName = &'static str;

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
        let existing_cfs: HashSet<String> = rocksdb::DB::list_cf(db_opts, path.de_unc())
            .unwrap_or_default()
            .into_iter()
            .collect();
        let requested_cfs: HashSet<String> =
            cfds.iter().map(|cfd| cfd.name().to_string()).collect();
        let missing_cfs: HashSet<&str> = requested_cfs
            .difference(&existing_cfs)
            .map(|cf| {
                warn!("Missing CF: {}", cf);
                cf.as_ref()
            })
            .collect();
        let unrecognized_cfs = existing_cfs.difference(&requested_cfs);

        let all_cfds = cfds
            .into_iter()
            .chain(unrecognized_cfs.map(Self::cfd_for_unrecognized_cf));

        let inner = {
            use rocksdb::DB;
            use OpenMode::*;

            match open_mode {
                ReadWrite => DB::open_cf_descriptors(db_opts, path.de_unc(), all_cfds),
                ReadOnly => {
                    DB::open_cf_descriptors_read_only(
                        db_opts,
                        path.de_unc(),
                        all_cfds.filter(|cfd| !missing_cfs.contains(cfd.name())),
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

    fn cfd_for_unrecognized_cf(cf: &String) -> ColumnFamilyDescriptor {
        warn!("Unrecognized CF: {}", cf);

        let mut cf_opts = Options::default();
        cf_opts.set_compression_type(DBCompressionType::Lz4);
        ColumnFamilyDescriptor::new(cf.to_string(), cf_opts)
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
        let k = <S::Key as KeyCodec<S>>::encode_key(schema_key)?;
        let cf_handle = self.get_cf_handle(S::COLUMN_FAMILY_NAME)?;

        let result = self.inner.get_cf(cf_handle, k).into_db_res()?;
        result
            .map(|raw_value| <S::Value as ValueCodec<S>>::decode_value(&raw_value))
            .transpose()
            .map_err(Into::into)
    }

    pub fn new_native_batch(&self) -> NativeBatch {
        NativeBatch::new(self)
    }

    /// Writes single record.
    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> DbResult<()> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        let mut batch = self.new_native_batch();
        batch.put::<S>(key, value)?;
        self.write_schemas(batch)
    }

    /// Deletes a single record.
    pub fn delete<S: Schema>(&self, key: &S::Key) -> DbResult<()> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        let mut batch = self.new_native_batch();
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

    fn write_schemas_inner(&self, batch: impl IntoRawBatch, option: &WriteOptions) -> DbResult<()> {
        let raw_batch = batch.into_raw_batch(self)?;

        let serialized_size = raw_batch.inner.size_in_bytes();
        self.inner
            .write_opt(raw_batch.inner, option)
            .into_db_res()?;

        raw_batch.stats.commit();
        Ok(())
    }

    /// Writes a group of records wrapped in a [`SchemaBatch`].
    pub fn write_schemas(&self, batch: impl IntoRawBatch) -> DbResult<()> {
        self.write_schemas_inner(batch, &sync_write_option())
    }

    /// Writes without sync flag in write option.
    /// If this flag is false, and the machine crashes, some recent
    /// writes may be lost.  Note that if it is just the process that
    /// crashes (i.e., the machine does not reboot), no writes will be
    /// lost even if sync==false.
    pub fn write_schemas_relaxed(&self, batch: impl IntoRawBatch) -> DbResult<()> {
        self.write_schemas_inner(batch, &WriteOptions::default())
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
fn sync_write_option() -> rocksdb::WriteOptions {
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
