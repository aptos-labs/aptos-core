// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{APTOS_SCHEMADB_DELETES_SAMPLED, APTOS_SCHEMADB_PUT_BYTES_SAMPLED, TIMER},
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName, DB,
};
use aptos_drop_helper::DropHelper;
use aptos_metrics_core::{IntCounterVecHelper, TimerHelper};
use aptos_storage_interface::Result as DbResult;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

#[derive(Debug, Default)]
pub struct BatchStats {
    put_sizes: HashMap<ColumnFamilyName, Vec<usize>>,
    num_deletes: HashMap<ColumnFamilyName, usize>,
}

impl BatchStats {
    fn put(&mut self, cf_name: ColumnFamilyName, size: usize) {
        self.put_sizes.entry(cf_name).or_default().push(size);
    }

    fn delete(&mut self, cf_name: ColumnFamilyName) {
        *self.num_deletes.entry(cf_name).or_default() += 1
    }

    fn commit(&self) {
        for (cf_name, put_sizes) in &self.put_sizes {
            for put_size in put_sizes {
                APTOS_SCHEMADB_PUT_BYTES_SAMPLED.observe_with(&[cf_name], *put_size as f64);
            }
        }
        for (cf_name, num_deletes) in &self.num_deletes {
            APTOS_SCHEMADB_DELETES_SAMPLED.inc_with_by(&[cf_name], *num_deletes as u64);
        }
    }
}

#[derive(Debug)]
pub struct SampledBatchStats {
    inner: Option<BatchStats>,
}

impl SampledBatchStats {
    pub fn put(&mut self, cf_name: ColumnFamilyName, size: usize) {
        if let Some(inner) = self.inner.as_mut() {
            inner.put(cf_name, size)
        }
    }

    pub fn delete(&mut self, cf_name: ColumnFamilyName) {
        if let Some(inner) = self.inner.as_mut() {
            inner.delete(cf_name)
        }
    }

    pub fn commit(&self) {
        if let Some(inner) = self.inner.as_ref() {
            inner.commit()
        }
    }
}

impl Default for SampledBatchStats {
    fn default() -> Self {
        const SAMPLING_PCT: usize = 1;

        Self {
            inner: (rand::random::<usize>() % 100 < SAMPLING_PCT).then_some(Default::default()),
        }
    }
}

#[derive(Default)]
pub struct RawBatch {
    pub inner: rocksdb::WriteBatch,
    pub stats: SampledBatchStats,
}

pub trait IntoRawBatch {
    fn into_raw_batch(self, db: &DB) -> DbResult<RawBatch>;
}

impl IntoRawBatch for RawBatch {
    fn into_raw_batch(self, _db: &DB) -> DbResult<RawBatch> {
        Ok(self)
    }
}

pub trait WriteBatch: IntoRawBatch {
    fn stats(&mut self) -> &mut SampledBatchStats;

    /// Adds an insert/update operation to the batch.
    fn put<S: Schema>(&mut self, key: &S::Key, value: &S::Value) -> DbResult<()> {
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;
        let value = <S::Value as ValueCodec<S>>::encode_value(value)?;

        self.stats()
            .put(S::COLUMN_FAMILY_NAME, key.len() + value.len());
        self.raw_put(S::COLUMN_FAMILY_NAME, key, value)
    }

    fn raw_put(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>, value: Vec<u8>) -> DbResult<()>;

    /// Adds a delete operation to the batch.
    fn delete<S: Schema>(&mut self, key: &S::Key) -> DbResult<()> {
        let key = <S::Key as KeyCodec<S>>::encode_key(key)?;

        self.stats().delete(S::COLUMN_FAMILY_NAME);
        self.raw_delete(S::COLUMN_FAMILY_NAME, key)
    }

    fn raw_delete(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>) -> DbResult<()>;
}

#[derive(Debug)]
pub enum WriteOp {
    Value { key: Vec<u8>, value: Vec<u8> },
    Deletion { key: Vec<u8> },
}

/// `SchemaBatch` holds a collection of updates that can be applied to a DB atomically. The updates
/// will be applied in the order in which they are added to the `SchemaBatch`.
#[derive(Debug, Default)]
pub struct SchemaBatch {
    rows: DropHelper<HashMap<ColumnFamilyName, Vec<WriteOp>>>,
    stats: SampledBatchStats,
}

impl SchemaBatch {
    /// Creates an empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// keep these on the struct itself so that we don't need to update each call site.
    pub fn put<S: Schema>(&mut self, key: &S::Key, value: &S::Value) -> DbResult<()> {
        <Self as WriteBatch>::put::<S>(self, key, value)
    }

    pub fn delete<S: Schema>(&mut self, key: &S::Key) -> DbResult<()> {
        <Self as WriteBatch>::delete::<S>(self, key)
    }
}

impl WriteBatch for SchemaBatch {
    fn stats(&mut self) -> &mut SampledBatchStats {
        &mut self.stats
    }

    fn raw_put(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>, value: Vec<u8>) -> DbResult<()> {
        self.rows
            .entry(cf_name)
            .or_default()
            .push(WriteOp::Value { key, value });

        Ok(())
    }

    fn raw_delete(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>) -> DbResult<()> {
        self.rows
            .entry(cf_name)
            .or_default()
            .push(WriteOp::Deletion { key });

        Ok(())
    }
}

impl IntoRawBatch for SchemaBatch {
    fn into_raw_batch(self, db: &DB) -> DbResult<RawBatch> {
        let _timer = TIMER.timer_with(&["schema_batch_to_raw_batch", &db.name]);

        let Self { rows, stats } = self;

        let mut db_batch = rocksdb::WriteBatch::default();
        for (cf_name, rows) in rows.iter() {
            let cf_handle = db.get_cf_handle(cf_name)?;
            for write_op in rows {
                match write_op {
                    WriteOp::Value { key, value } => db_batch.put_cf(cf_handle, key, value),
                    WriteOp::Deletion { key } => db_batch.delete_cf(cf_handle, key),
                }
            }
        }

        Ok(RawBatch {
            inner: db_batch,
            stats,
        })
    }
}

/// Similar to SchemaBatch, but wraps around rocksdb::WriteBatch directly.
/// For that to work, a reference to the DB needs to be held.
pub struct NativeBatch<'db> {
    db: &'db DB,
    raw_batch: RawBatch,
}

impl Debug for NativeBatch<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeBatch for DB {} ", self.db.name)
    }
}

impl<'db> NativeBatch<'db> {
    /// Creates an empty batch.
    pub fn new(db: &'db DB) -> Self {
        Self {
            db,
            raw_batch: RawBatch::default(),
        }
    }
}

impl WriteBatch for NativeBatch<'_> {
    fn stats(&mut self) -> &mut SampledBatchStats {
        &mut self.raw_batch.stats
    }

    fn raw_put(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>, value: Vec<u8>) -> DbResult<()> {
        self.raw_batch
            .inner
            .put_cf(&self.db.get_cf_handle(cf_name)?, &key, &value);

        Ok(())
    }

    fn raw_delete(&mut self, cf_name: ColumnFamilyName, key: Vec<u8>) -> DbResult<()> {
        self.raw_batch
            .inner
            .delete_cf(&self.db.get_cf_handle(cf_name)?, &key);

        Ok(())
    }
}

impl IntoRawBatch for NativeBatch<'_> {
    fn into_raw_batch(self, _db: &DB) -> DbResult<RawBatch> {
        let Self { db: _, raw_batch } = self;

        Ok(raw_batch)
    }
}
