// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod consensusdb_test;
mod schema;

pub use crate::consensusdb::schema::dag::{
    DAG0_CERTIFIED_NODE_CF_NAME, DAG0_NODE_CF_NAME, DAG0_VOTE_CF_NAME, DAG1_CERTIFIED_NODE_CF_NAME,
    DAG1_NODE_CF_NAME, DAG1_VOTE_CF_NAME, DAG2_CERTIFIED_NODE_CF_NAME, DAG2_NODE_CF_NAME,
    DAG2_VOTE_CF_NAME,
};
use crate::error::DbError;
use anyhow::Result;
use aptos_consensus_types::{block::Block, quorum_cert::QuorumCert};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use aptos_schemadb::{
    schema::Schema, BlockBasedOptions, Cache, ColumnFamilyDescriptor, ColumnFamilyName,
    DBCompressionType, Options, ReadOptions, SchemaBatch, DB, DEFAULT_COLUMN_FAMILY_NAME,
};
use aptos_storage_interface::AptosDbError;
use once_cell::sync::Lazy;
pub use schema::{
    block::BlockSchema,
    dag::{
        Dag0CertifiedNodeSchema, Dag0NodeSchema, Dag0VoteSchema, Dag1CertifiedNodeSchema,
        Dag1NodeSchema, Dag1VoteSchema, Dag2CertifiedNodeSchema, Dag2NodeSchema, Dag2VoteSchema,
    },
    quorum_certificate::QCSchema,
};
use schema::{
    single_entry::{SingleEntryKey, SingleEntrySchema},
    BLOCK_CF_NAME, QC_CF_NAME, SINGLE_ENTRY_CF_NAME,
};
use std::{
    collections::HashMap,
    iter::Iterator,
    path::Path,
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
// use crate::consensusdb::schema::DAG2_NODE_CF_NAME;

/// The name of the consensus db file
pub const CONSENSUS_DB_NAME: &str = "consensus_db";

/// Creates new physical DB checkpoint in directory specified by `checkpoint_path`.
pub fn create_checkpoint<P: AsRef<Path> + Clone>(db_path: P, checkpoint_path: P) -> Result<()> {
    let start = Instant::now();
    let consensus_db_checkpoint_path = checkpoint_path.as_ref().join(CONSENSUS_DB_NAME);
    std::fs::remove_dir_all(&consensus_db_checkpoint_path).unwrap_or(());
    ConsensusDB::new(db_path)
        .db
        .create_checkpoint(&consensus_db_checkpoint_path)?;
    info!(
        path = consensus_db_checkpoint_path,
        time_ms = %start.elapsed().as_millis(),
        "Made ConsensusDB checkpoint."
    );
    Ok(())
}

pub struct ConsensusDB {
    db: DB,
}

impl ConsensusDB {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            /* UNUSED CF = */ DEFAULT_COLUMN_FAMILY_NAME,
            BLOCK_CF_NAME,
            QC_CF_NAME,
            SINGLE_ENTRY_CF_NAME,
            DAG0_NODE_CF_NAME,
            DAG1_NODE_CF_NAME,
            DAG2_NODE_CF_NAME,
            DAG0_CERTIFIED_NODE_CF_NAME,
            DAG1_CERTIFIED_NODE_CF_NAME,
            DAG2_CERTIFIED_NODE_CF_NAME,
            DAG0_VOTE_CF_NAME,
            DAG1_VOTE_CF_NAME,
            DAG2_VOTE_CF_NAME,
            "ordered_anchor_id", // deprecated CF
        ];

        let path = db_root_path.as_ref().join(CONSENSUS_DB_NAME);
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
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
        let db = DB::open_cf(&opts, path.clone(), "consensus", cfds)
            .expect("ConsensusDB open failed; unable to continue");

        info!(
            "Opened ConsensusDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }

    pub fn get_data(
        &self,
    ) -> Result<(
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Vec<Block>,
        Vec<QuorumCert>,
    )> {
        let last_vote = self.get_last_vote()?;
        let highest_2chain_timeout_certificate = self.get_highest_2chain_timeout_certificate()?;
        let consensus_blocks = self
            .get_all::<BlockSchema>()?
            .into_iter()
            .map(|(_, block)| block)
            .collect();
        let consensus_qcs = self
            .get_all::<QCSchema>()?
            .into_iter()
            .map(|(_, qc)| qc)
            .collect();
        Ok((
            last_vote,
            highest_2chain_timeout_certificate,
            consensus_blocks,
            consensus_qcs,
        ))
    }

    pub fn save_highest_2chain_timeout_certificate(&self, tc: Vec<u8>) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.put::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert, &tc)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn save_vote(&self, last_vote: Vec<u8>) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.put::<SingleEntrySchema>(&SingleEntryKey::LastVote, &last_vote)?;
        self.commit(batch)
    }

    pub fn save_blocks_and_quorum_certificates(
        &self,
        block_data: Vec<Block>,
        qc_data: Vec<QuorumCert>,
    ) -> Result<(), DbError> {
        if block_data.is_empty() && qc_data.is_empty() {
            return Err(anyhow::anyhow!("Consensus block and qc data is empty!").into());
        }
        let batch = SchemaBatch::new();
        block_data
            .iter()
            .try_for_each(|block| batch.put::<BlockSchema>(&block.id(), block))?;
        qc_data
            .iter()
            .try_for_each(|qc| batch.put::<QCSchema>(&qc.certified_block().id(), qc))?;
        self.commit(batch)
    }

    pub fn delete_blocks_and_quorum_certificates(
        &self,
        block_ids: Vec<HashValue>,
    ) -> Result<(), DbError> {
        if block_ids.is_empty() {
            return Err(anyhow::anyhow!("Consensus block ids is empty!").into());
        }
        let batch = SchemaBatch::new();
        block_ids.iter().try_for_each(|hash| {
            batch.delete::<BlockSchema>(hash)?;
            batch.delete::<QCSchema>(hash)
        })?;
        self.commit(batch)
    }

    /// Write the whole schema batch including all data necessary to mutate the ledger
    /// state of some transaction by leveraging rocksdb atomicity support.
    fn commit(&self, batch: SchemaBatch) -> Result<(), DbError> {
        self.db.write_schemas(batch)?;
        Ok(())
    }

    /// Get latest timeout certificates (we only store the latest highest timeout certificates).
    fn get_highest_2chain_timeout_certificate(&self) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .db
            .get::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert)?)
    }

    pub fn delete_highest_2chain_timeout_certificate(&self) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.delete::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert)?;
        self.commit(batch)
    }

    /// Get serialized latest vote (if available)
    fn get_last_vote(&self) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .db
            .get::<SingleEntrySchema>(&SingleEntryKey::LastVote)?)
    }

    pub fn delete_last_vote_msg(&self) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.delete::<SingleEntrySchema>(&SingleEntryKey::LastVote)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn delete<S: Schema>(&self, keys: Vec<S::Key>) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        keys.iter().try_for_each(|key| batch.delete::<S>(key))?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn get_all<S: Schema>(&self) -> Result<Vec<(S::Key, S::Value)>, DbError> {
        let mut opts = ReadOptions::default();
        opts.set_async_io(true);
        opts.set_readahead_size(1 * (1 << 20));
        let mut iter = self.db.iter::<S>(opts)?;
        iter.seek_to_first();
        Ok(iter.collect::<Result<Vec<(S::Key, S::Value)>, AptosDbError>>()?)
    }

    pub fn get<S: Schema>(&self, key: &S::Key) -> Result<Option<S::Value>, DbError> {
        Ok(self.db.get::<S>(key)?)
    }
}

#[derive(Debug)]
pub(crate) struct RocksdbPropertyReporter {
    sender: Mutex<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl RocksdbPropertyReporter {
    pub(crate) fn new(consensus_db: Arc<ConsensusDB>) -> Self {
        let (send, recv) = mpsc::channel();
        let join_handle = Some(thread::spawn(move || loop {
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

fn update_rocksdb_properties(consensus_db: &ConsensusDB) -> Result<()> {
    // let _timer = OTHER_TIMERS_SECONDS
    //     .with_label_values(&["update_rocksdb_properties"])
    //     .start_timer();

    for cf in consensus_db_column_families() {
        set_property(cf, &consensus_db.db)?;
    }

    Ok(())
}

pub(super) fn consensus_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        DEFAULT_COLUMN_FAMILY_NAME,
        BLOCK_CF_NAME,
        QC_CF_NAME,
        SINGLE_ENTRY_CF_NAME,
        DAG0_NODE_CF_NAME,
        DAG0_CERTIFIED_NODE_CF_NAME,
        DAG0_VOTE_CF_NAME,
        DAG1_NODE_CF_NAME,
        DAG1_CERTIFIED_NODE_CF_NAME,
        DAG1_VOTE_CF_NAME,
        DAG2_NODE_CF_NAME,
        DAG2_CERTIFIED_NODE_CF_NAME,
        DAG2_VOTE_CF_NAME,
        "ordered_anchor_id",
    ]
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
