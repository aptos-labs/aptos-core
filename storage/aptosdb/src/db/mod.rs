// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::backup_handler::BackupHandler, event_store::EventStore, ledger_db::LedgerDb,
    pruner::LedgerPrunerManager, rocksdb_property_reporter::RocksdbPropertyReporter,
    state_kv_db::StateKvDb, state_merkle_db::StateMerkleDb, state_store::StateStore,
    transaction_store::TransactionStore,
};
use aptos_config::config::{PrunerConfig, RocksdbConfigs, StorageDirPaths};
use aptos_db_indexer::{db_indexer::InternalIndexerDB, Indexer};
use aptos_logger::prelude::*;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use std::{path::Path, sync::Arc, time::Instant};
use tokio::sync::watch::Sender;

#[cfg(test)]
mod aptosdb_test;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test_helper;

/// This holds a handle to the underlying DB responsible for physical storage and provides APIs for
/// access to the core Aptos data structures.
pub struct AptosDB {
    pub(crate) ledger_db: Arc<LedgerDb>,
    pub(crate) state_kv_db: Arc<StateKvDb>,
    pub(crate) event_store: Arc<EventStore>,
    pub(crate) state_store: Arc<StateStore>,
    pub(crate) transaction_store: Arc<TransactionStore>,
    ledger_pruner: LedgerPrunerManager,
    _rocksdb_property_reporter: RocksdbPropertyReporter,
    /// This is just to detect concurrent calls to `pre_commit_ledger()`
    pre_commit_lock: std::sync::Mutex<()>,
    /// This is just to detect concurrent calls to `commit_ledger()`
    commit_lock: std::sync::Mutex<()>,
    indexer: Option<Indexer>,
    skip_index_and_usage: bool,
    update_subscriber: Option<Sender<(Instant, Version)>>,
}

// DbReader implementations and private functions used by them.
mod aptosdb_reader;
// DbWriter implementations and private functions used by them.
mod aptosdb_writer;
// Other private methods.
mod aptosdb_internal;
// Testonly methods.
#[cfg(any(test, feature = "fuzzing", feature = "consensus-only-perf-test"))]
mod aptosdb_testonly;

#[cfg(feature = "consensus-only-perf-test")]
pub mod fake_aptosdb;

impl AptosDB {
    pub fn open(
        db_paths: StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            false,
            internal_indexer_db,
        )
    }

    pub fn open_kv_only(
        db_paths: StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            true,
            internal_indexer_db,
        )
    }

    pub fn open_dbs(
        db_paths: &StorageDirPaths,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        max_num_nodes_per_lru_cache_shard: usize,
    ) -> Result<(LedgerDb, StateMerkleDb, StateKvDb)> {
        let ledger_db = LedgerDb::new(db_paths.ledger_db_root_path(), rocksdb_configs, readonly)?;
        let state_kv_db = StateKvDb::new(
            db_paths,
            rocksdb_configs,
            readonly,
            ledger_db.metadata_db_arc(),
        )?;
        let state_merkle_db = StateMerkleDb::new(
            db_paths,
            rocksdb_configs,
            readonly,
            max_num_nodes_per_lru_cache_shard,
        )?;

        Ok((ledger_db, state_merkle_db, state_kv_db))
    }

    pub fn add_version_update_subscriber(
        &mut self,
        sender: Sender<(Instant, Version)>,
    ) -> Result<()> {
        self.update_subscriber = Some(sender);
        Ok(())
    }

    /// Gets an instance of `BackupHandler` for data backup purpose.
    pub fn get_backup_handler(&self) -> BackupHandler {
        BackupHandler::new(Arc::clone(&self.state_store), Arc::clone(&self.ledger_db))
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint(
        db_path: impl AsRef<Path>,
        cp_path: impl AsRef<Path>,
        sharding: bool,
    ) -> Result<()> {
        let start = Instant::now();

        info!(sharding = sharding, "Creating checkpoint for AptosDB.");

        LedgerDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref(), sharding)?;
        if sharding {
            StateKvDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref())?;
        }
        StateMerkleDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref(), sharding)?;

        info!(
            db_path = db_path.as_ref(),
            cp_path = cp_path.as_ref(),
            time_ms = %start.elapsed().as_millis(),
            "Made AptosDB checkpoint."
        );
        Ok(())
    }

    pub fn commit_genesis_ledger_info(&self, genesis_li: &LedgerInfoWithSignatures) -> Result<()> {
        let ledger_metadata_db = self.ledger_db.metadata_db();
        let current_epoch = ledger_metadata_db
            .get_latest_ledger_info_option()
            .map_or(0, |li| li.ledger_info().next_block_epoch());
        ensure!(
            genesis_li.ledger_info().epoch() == current_epoch && current_epoch == 0,
            "Genesis ledger info epoch is not 0"
        );
        let mut ledger_batch = SchemaBatch::new();
        ledger_metadata_db.put_ledger_info(genesis_li, &mut ledger_batch)?;
        ledger_metadata_db.write_schemas(ledger_batch)
    }
}
