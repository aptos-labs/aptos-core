// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::DbError,
    quorum_store::{
        schema::{BatchIdSchema, BatchSchema, BATCH_CF_NAME, BATCH_ID_CF_NAME},
        types::PersistedValue,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_schemadb::{
    batch::{SchemaBatch, WriteBatch},
    schema::Schema,
    Options, DB,
};
use aptos_types::quorum_store::BatchId;
use std::{collections::HashMap, path::Path, time::Instant};

pub trait QuorumStoreStorage: Sync + Send {
    fn delete_batches(&self, digests: Vec<HashValue>) -> Result<(), DbError>;

    fn get_all_batches(&self) -> Result<HashMap<HashValue, PersistedValue>>;

    fn save_batch(&self, batch: PersistedValue) -> Result<(), DbError>;

    fn get_batch(&self, digest: &HashValue) -> Result<Option<PersistedValue>, DbError>;

    fn delete_batch_id(&self, epoch: u64) -> Result<(), DbError>;

    fn clean_and_get_batch_id(&self, current_epoch: u64) -> Result<Option<BatchId>, DbError>;

    fn save_batch_id(&self, epoch: u64, batch_id: BatchId) -> Result<(), DbError>;
}

/// The name of the quorum store db file
pub const QUORUM_STORE_DB_NAME: &str = "quorumstoreDB";

pub struct QuorumStoreDB {
    db: DB,
}

impl QuorumStoreDB {
    pub(crate) fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![BATCH_CF_NAME, BATCH_ID_CF_NAME];

        // TODO: this fails twins tests because it assumes a unique path per process
        let path = db_root_path.as_ref().join(QUORUM_STORE_DB_NAME);
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), QUORUM_STORE_DB_NAME, column_families, &opts)
            .expect("QuorumstoreDB open failed; unable to continue");

        info!(
            "Opened QuorumstoreDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }

    /// Relaxed writes instead of sync writes.
    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<(), DbError> {
        // Not necessary to use a batch, but we'd like a central place to bump counters.
        let mut batch = self.db.new_native_batch();
        batch.put::<S>(key, value)?;
        self.db.write_schemas_relaxed(batch)?;
        Ok(())
    }
}

impl QuorumStoreStorage for QuorumStoreDB {
    fn delete_batches(&self, digests: Vec<HashValue>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        for digest in digests.iter() {
            trace!("QS: db delete digest {}", digest);
            batch.delete::<BatchSchema>(digest)?;
        }
        self.db.write_schemas_relaxed(batch)?;
        Ok(())
    }

    fn get_all_batches(&self) -> Result<HashMap<HashValue, PersistedValue>> {
        let mut iter = self.db.iter::<BatchSchema>()?;
        iter.seek_to_first();
        iter.map(|res| res.map_err(Into::into))
            .collect::<Result<HashMap<HashValue, PersistedValue>>>()
    }

    fn save_batch(&self, batch: PersistedValue) -> Result<(), DbError> {
        trace!(
            "QS: db persists digest {} expiration {:?}",
            batch.digest(),
            batch.expiration()
        );
        self.put::<BatchSchema>(batch.digest(), &batch)
    }

    fn get_batch(&self, digest: &HashValue) -> Result<Option<PersistedValue>, DbError> {
        Ok(self.db.get::<BatchSchema>(digest)?)
    }

    fn delete_batch_id(&self, epoch: u64) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.delete::<BatchIdSchema>(&epoch)?;
        self.db.write_schemas_relaxed(batch)?;
        Ok(())
    }

    fn clean_and_get_batch_id(&self, current_epoch: u64) -> Result<Option<BatchId>, DbError> {
        let mut iter = self.db.iter::<BatchIdSchema>()?;
        iter.seek_to_first();
        let epoch_batch_id = iter
            .map(|res| res.map_err(Into::into))
            .collect::<Result<HashMap<u64, BatchId>>>()?;
        let mut ret = None;
        for (epoch, batch_id) in epoch_batch_id {
            assert!(current_epoch >= epoch);
            if epoch < current_epoch {
                self.delete_batch_id(epoch)?;
            } else {
                ret = Some(batch_id);
            }
        }
        Ok(ret)
    }

    fn save_batch_id(&self, epoch: u64, batch_id: BatchId) -> Result<(), DbError> {
        self.put::<BatchIdSchema>(&epoch, &batch_id)
    }
}

#[cfg(test)]
pub(crate) use mock::MockQuorumStoreDB;

#[cfg(test)]
pub mod mock {
    use super::*;
    pub struct MockQuorumStoreDB {}

    impl MockQuorumStoreDB {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Default for MockQuorumStoreDB {
        fn default() -> Self {
            Self::new()
        }
    }

    impl QuorumStoreStorage for MockQuorumStoreDB {
        fn delete_batches(&self, _: Vec<HashValue>) -> Result<(), DbError> {
            Ok(())
        }

        fn get_all_batches(&self) -> Result<HashMap<HashValue, PersistedValue>> {
            Ok(HashMap::new())
        }

        fn save_batch(&self, _: PersistedValue) -> Result<(), DbError> {
            Ok(())
        }

        fn get_batch(&self, _: &HashValue) -> Result<Option<PersistedValue>, DbError> {
            Ok(None)
        }

        fn delete_batch_id(&self, _: u64) -> Result<(), DbError> {
            Ok(())
        }

        fn clean_and_get_batch_id(&self, _: u64) -> Result<Option<BatchId>, DbError> {
            Ok(Some(BatchId::new_for_test(0)))
        }

        fn save_batch_id(&self, _: u64, _: BatchId) -> Result<(), DbError> {
            Ok(())
        }
    }
}
