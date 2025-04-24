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
use aptos_consensus_types::proof_of_store::BatchId;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use aptos_schemadb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, ColumnFamilyName, DBCompressionType, Options,
    SchemaBatch, DB, DEFAULT_COLUMN_FAMILY_NAME,
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    path::Path,
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

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
        opts.set_enable_blob_files(true);
        opts.set_min_blob_size(16 * 1024);

        opts.set_blob_file_size(256 * 1024 * 1024);
        opts.set_write_buffer_size(256 * 1024 * 1024);

        opts.increase_parallelism(16);
        opts.set_max_background_jobs(8);
        opts.set_enable_write_thread_adaptive_yield(true);
        opts.set_allow_concurrent_memtable_write(true);
        opts.set_enable_pipelined_write(true);

        let mut table_options = BlockBasedOptions::default();
        table_options.set_block_size(32 * (1 << 10)); // 4KB
        let cache = Cache::new_lru_cache(2 * (1 << 30)); // 1GB
        table_options.set_block_cache(&cache);
        let cfds = column_families
            .iter()
            .map(|cf_name| {
                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(DBCompressionType::Lz4);
                cf_opts.set_block_based_table_factory(&table_options);
                ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts)
            })
            .collect();

        let db = DB::open_cf(&opts, path.clone(), QUORUM_STORE_DB_NAME, cfds)
            .expect("QuorumstoreDB open failed; unable to continue");

        info!(
            "Opened QuorumstoreDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }
}

impl QuorumStoreStorage for QuorumStoreDB {
    fn delete_batches(&self, digests: Vec<HashValue>) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        for digest in digests.iter() {
            trace!("QS: db delete digest {}", digest);
            batch.delete::<BatchSchema>(digest)?;
        }
        self.db.write_schemas(batch)?;
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
        self.db.put::<BatchSchema>(batch.digest(), &batch)?;
        Ok(())
    }

    fn get_batch(&self, digest: &HashValue) -> Result<Option<PersistedValue>, DbError> {
        Ok(self.db.get::<BatchSchema>(digest)?)
    }

    fn delete_batch_id(&self, epoch: u64) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.delete::<BatchIdSchema>(&epoch)?;
        self.db.write_schemas(batch)?;
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
        self.db.put::<BatchIdSchema>(&epoch, &batch_id)?;
        Ok(())
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

#[derive(Debug)]
pub(crate) struct RocksdbPropertyReporter {
    sender: Mutex<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl RocksdbPropertyReporter {
    pub(crate) fn new(consensus_db: Arc<QuorumStoreDB>) -> Self {
        let (send, recv) = mpsc::channel();
        let join_handle = Some(std::thread::spawn(move || loop {
            if let Err(e) = update_rocksdb_properties(&consensus_db) {
                warn!(
                    error = ?e,
                    "Updating rocksdb property failed."
                );
            }
            // report rocksdb properties each 10 seconds
            const TIMEOUT_MS: u64 = if cfg!(test) { 10 } else { 10000 };

            match recv.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
                Ok(_) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }));
        Self {
            sender: Mutex::new(send),
            join_handle,
        }
    }
}

impl Drop for RocksdbPropertyReporter {
    fn drop(&mut self) {
        // Notify the property reporting thread to exit
        self.sender.lock().send(()).unwrap();
        self.join_handle
            .take()
            .expect("Rocksdb property reporting thread must exist.")
            .join()
            .expect("Rocksdb property reporting thread should join peacefully.");
    }
}

fn update_rocksdb_properties(consensus_db: &QuorumStoreDB) -> Result<()> {
    // let _timer = OTHER_TIMERS_SECONDS
    //     .with_label_values(&["update_rocksdb_properties"])
    //     .start_timer();

    for cf in quorum_store_db_column_families() {
        set_property(cf, &consensus_db.db)?;
    }

    Ok(())
}

pub(super) fn quorum_store_db_column_families() -> Vec<ColumnFamilyName> {
    vec![DEFAULT_COLUMN_FAMILY_NAME, BATCH_CF_NAME, BATCH_ID_CF_NAME]
}

fn set_property(cf_name: &str, db: &DB) -> Result<()> {
    for (rockdb_property_name, aptos_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
        DAG_ROCKSDB_PROPERTIES
            .with_label_values(&[cf_name, aptos_rocksdb_property_name])
            .set(db.get_property(cf_name, rockdb_property_name)? as i64);
    }
    Ok(())
}

/// Rocksdb metrics
static DAG_ROCKSDB_PROPERTIES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        // metric name
        "aptos_dag_rocksdb_properties",
        // metric description
        "rocksdb integer properties",
        // metric labels (dimensions)
        &["cf_name", "property_name",]
    )
    .unwrap()
});

static ROCKSDB_PROPERTY_MAP: Lazy<HashMap<&str, String>> = Lazy::new(|| {
    [
        "rocksdb.num-immutable-mem-table",
        "rocksdb.mem-table-flush-pending",
        "rocksdb.compaction-pending",
        "rocksdb.background-errors",
        "rocksdb.cur-size-active-mem-table",
        "rocksdb.cur-size-all-mem-tables",
        "rocksdb.size-all-mem-tables",
        "rocksdb.num-entries-active-mem-table",
        "rocksdb.num-entries-imm-mem-tables",
        "rocksdb.num-deletes-active-mem-table",
        "rocksdb.num-deletes-imm-mem-tables",
        "rocksdb.estimate-num-keys",
        "rocksdb.estimate-table-readers-mem",
        "rocksdb.is-file-deletions-enabled",
        "rocksdb.num-snapshots",
        "rocksdb.oldest-snapshot-time",
        "rocksdb.num-live-versions",
        "rocksdb.current-super-version-number",
        "rocksdb.estimate-live-data-size",
        "rocksdb.min-log-number-to-keep",
        "rocksdb.min-obsolete-sst-number-to-keep",
        "rocksdb.total-sst-files-size",
        "rocksdb.live-sst-files-size",
        "rocksdb.base-level",
        "rocksdb.estimate-pending-compaction-bytes",
        "rocksdb.num-running-compactions",
        "rocksdb.num-running-flushes",
        "rocksdb.actual-delayed-write-rate",
        "rocksdb.is-write-stopped",
        "rocksdb.block-cache-capacity",
        "rocksdb.block-cache-usage",
        "rocksdb.block-cache-pinned-usage",
    ]
    .iter()
    .map(|x| (*x, format!("aptos_{}", x.replace('.', "_"))))
    .collect()
});
