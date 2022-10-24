// Copyright (c) Aptos
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
        APTOS_SCHEMADB_BATCH_PUT_LATENCY_SECONDS, APTOS_SCHEMADB_DELETES, APTOS_SCHEMADB_GET_BYTES,
        APTOS_SCHEMADB_GET_LATENCY_SECONDS, APTOS_SCHEMADB_ITER_BYTES,
        APTOS_SCHEMADB_ITER_LATENCY_SECONDS, APTOS_SCHEMADB_PUT_BYTES,
    },
    schema::{KeyCodec, Schema, SeekKeyCodec, ValueCodec},
};
use anyhow::{format_err, Result};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use std::{collections::HashMap, iter::Iterator, path::Path};

use iterator::{ScanDirection, SchemaIterator};
/// Type alias to `rocksdb::ReadOptions`. See [`rocksdb doc`](https://github.com/pingcap/rust-rocksdb/blob/master/src/rocksdb_options.rs)
pub use rocksdb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, DBCompressionType, Options, ReadOptions,
    SliceTransform, DEFAULT_COLUMN_FAMILY_NAME,
};

pub type ColumnFamilyName = &'static str;

#[derive(Debug)]
enum WriteOp {
    Value { key: Vec<u8>, value: Vec<u8> },
    Deletion { key: Vec<u8> },
}

/// `SchemaBatch` holds a collection of updates that can be applied to a DB atomically. The updates
/// will be applied in the order in which they are added to the `SchemaBatch`.
#[derive(Debug)]
pub struct SchemaBatch {
    rows: Mutex<HashMap<ColumnFamilyName, Vec<WriteOp>>>,
}

impl Default for SchemaBatch {
    fn default() -> Self {
        Self {
            rows: Mutex::new(HashMap::new()),
        }
    }
}

impl SchemaBatch {
    /// Creates an empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an insert/update operation to the batch.
    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<()> {
        let _timer = APTOS_SCHEMADB_BATCH_PUT_LATENCY_SECONDS
            .with_label_values(&["unknown"])
            .start_timer();
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;
        let value = <S::Value as ValueCodec<S>>::encode_value(value)?;
        self.rows
            .lock()
            .entry(S::COLUMN_FAMILY_NAME)
            .or_insert_with(Vec::new)
            .push(WriteOp::Value { key, value });

        Ok(())
    }

    /// Adds a delete operation to the batch.
    pub fn delete<S: Schema>(&self, key: &S::Key) -> Result<()> {
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;
        self.rows
            .lock()
            .entry(S::COLUMN_FAMILY_NAME)
            .or_insert_with(Vec::new)
            .push(WriteOp::Deletion { key });

        Ok(())
    }
}

/// This DB is a schematized RocksDB wrapper where all data passed in and out are typed according to
/// [`Schema`]s.
#[derive(Debug)]
pub struct DB {
    name: &'static str, // for logging
    inner: rocksdb::DB,
}

impl DB {
    pub fn open(
        path: impl AsRef<Path>,
        name: &'static str,
        column_families: Vec<ColumnFamilyName>,
        db_opts: &rocksdb::Options,
    ) -> Result<Self> {
        let db = DB::open_cf(
            db_opts,
            path,
            name,
            column_families
                .iter()
                .map(|cf_name| {
                    let mut cf_opts = rocksdb::Options::default();
                    cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                    rocksdb::ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts)
                })
                .collect(),
        )?;
        Ok(db)
    }

    pub fn open_cf(
        db_opts: &rocksdb::Options,
        path: impl AsRef<Path>,
        name: &'static str,
        cfds: Vec<rocksdb::ColumnFamilyDescriptor>,
    ) -> Result<DB> {
        let inner = rocksdb::DB::open_cf_descriptors(db_opts, path, cfds)?;
        Ok(Self::log_construct(name, inner))
    }

    /// Open db in readonly mode
    /// Note that this still assumes there's only one process that opens the same DB.
    /// See `open_as_secondary`
    pub fn open_cf_readonly(
        opts: &rocksdb::Options,
        path: impl AsRef<Path>,
        name: &'static str,
        cfs: Vec<ColumnFamilyName>,
    ) -> Result<DB> {
        let error_if_log_file_exists = false;
        let inner = rocksdb::DB::open_cf_for_read_only(opts, path, &cfs, error_if_log_file_exists)?;

        Ok(Self::log_construct(name, inner))
    }

    pub fn open_cf_as_secondary<P: AsRef<Path>>(
        opts: &rocksdb::Options,
        primary_path: P,
        secondary_path: P,
        name: &'static str,
        cfs: Vec<ColumnFamilyName>,
    ) -> Result<DB> {
        let inner = rocksdb::DB::open_cf_as_secondary(opts, primary_path, secondary_path, &cfs)?;
        Ok(Self::log_construct(name, inner))
    }

    fn log_construct(name: &'static str, inner: rocksdb::DB) -> DB {
        info!(rocksdb_name = name, "Opened RocksDB.");
        DB { name, inner }
    }

    /// Reads single record by key.
    pub fn get<S: Schema>(&self, schema_key: &S::Key) -> Result<Option<S::Value>> {
        let _timer = APTOS_SCHEMADB_GET_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .start_timer();

        let k = <S::Key as KeyCodec<S>>::encode_key(schema_key)?;
        let cf_handle = self.get_cf_handle(S::COLUMN_FAMILY_NAME)?;

        let result = self.inner.get_cf(cf_handle, &k)?;
        APTOS_SCHEMADB_GET_BYTES
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .observe(result.as_ref().map_or(0.0, |v| v.len() as f64));

        result
            .map(|raw_value| <S::Value as ValueCodec<S>>::decode_value(&raw_value))
            .transpose()
    }

    /// Writes single record.
    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<()> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        // Used in tests only anyway.
        let batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.write_schemas(batch)
    }

    fn iter_with_direction<S: Schema>(
        &self,
        opts: ReadOptions,
        direction: ScanDirection,
    ) -> Result<SchemaIterator<S>> {
        let cf_handle = self.get_cf_handle(S::COLUMN_FAMILY_NAME)?;
        Ok(SchemaIterator::new(
            self.inner.raw_iterator_cf_opt(cf_handle, opts),
            direction,
        ))
    }

    /// Returns a forward [`SchemaIterator`] on a certain schema.
    pub fn iter<S: Schema>(&self, opts: ReadOptions) -> Result<SchemaIterator<S>> {
        self.iter_with_direction::<S>(opts, ScanDirection::Forward)
    }

    /// Returns a backward [`SchemaIterator`] on a certain schema.
    pub fn rev_iter<S: Schema>(&self, opts: ReadOptions) -> Result<SchemaIterator<S>> {
        self.iter_with_direction::<S>(opts, ScanDirection::Backward)
    }

    /// Writes a group of records wrapped in a [`SchemaBatch`].
    pub fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        let _timer = APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS
            .with_label_values(&[self.name])
            .start_timer();
        let rows_locked = batch.rows.lock();

        let mut db_batch = rocksdb::WriteBatch::default();
        for (cf_name, rows) in rows_locked.iter() {
            let cf_handle = self.get_cf_handle(cf_name)?;
            for write_op in rows {
                match write_op {
                    WriteOp::Value { key, value } => db_batch.put_cf(cf_handle, key, value),
                    WriteOp::Deletion { key } => db_batch.delete_cf(cf_handle, key),
                }
            }
        }
        let serialized_size = db_batch.size_in_bytes();

        self.inner.write_opt(db_batch, &default_write_options())?;

        // Bump counters only after DB write succeeds.
        for (cf_name, rows) in rows_locked.iter() {
            for write_op in rows {
                match write_op {
                    WriteOp::Value { key, value } => {
                        APTOS_SCHEMADB_PUT_BYTES
                            .with_label_values(&[cf_name])
                            .observe((key.len() + value.len()) as f64);
                    }
                    WriteOp::Deletion { key: _ } => {
                        APTOS_SCHEMADB_DELETES.with_label_values(&[cf_name]).inc();
                    }
                }
            }
        }
        APTOS_SCHEMADB_BATCH_COMMIT_BYTES
            .with_label_values(&[self.name])
            .observe(serialized_size as f64);

        Ok(())
    }

    fn get_cf_handle(&self, cf_name: &str) -> Result<&rocksdb::ColumnFamily> {
        self.inner.cf_handle(cf_name).ok_or_else(|| {
            format_err!(
                "DB::cf_handle not found for column family name: {}",
                cf_name
            )
        })
    }

    /// Flushes memtable data. This is only used for testing `get_approximate_sizes_cf` in unit
    /// tests.
    pub fn flush_cf(&self, cf_name: &str) -> Result<()> {
        Ok(self.inner.flush_cf(self.get_cf_handle(cf_name)?)?)
    }

    pub fn get_property(&self, cf_name: &str, property_name: &str) -> Result<u64> {
        self.inner
            .property_int_value_cf(self.get_cf_handle(cf_name)?, property_name)?
            .ok_or_else(|| {
                format_err!(
                    "Unable to get property \"{}\" of  column family \"{}\".",
                    property_name,
                    cf_name,
                )
            })
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        rocksdb::checkpoint::Checkpoint::new(&self.inner)?.create_checkpoint(path)?;
        Ok(())
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
