// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup::backup_handler::BackupHandler, db::aptosdb_native_position::PositionBundle,
    event_store::EventStore, ledger_db::LedgerDb, position_db::PositionDb,
    position_merkle_db::PositionMerkleDb, pruner::LedgerPrunerManager,
    rocksdb_property_reporter::RocksdbPropertyReporter, state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb, state_store::StateStore, transaction_store::TransactionStore,
};
use aptos_config::config::{HotStateConfig, PrunerConfig, RocksdbConfigs, StorageDirPaths};
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_logger::prelude::*;
use aptos_schemadb::{batch::SchemaBatch, Cache, Env};
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
    pub(crate) hot_state_kv_db: Arc<StateKvDb>,
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
    update_subscriber: Option<Sender<(Instant, Version)>>,
    pub(crate) position: Option<Arc<PositionBundle>>,
}

// DbReader implementations and private functions used by them.
mod aptosdb_reader;
// DbWriter implementations and private functions used by them.
pub(crate) mod aptosdb_writer;
// Other private methods.
mod aptosdb_internal;
// Native-position subsystem: `PositionBundle` + `impl AptosDB` for
// `position()` / `native_state_committer()` / `init_native_position()`.
mod aptosdb_native_position;
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
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
        hot_state_config: HotStateConfig,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            false,
            internal_indexer_db,
            hot_state_config,
        )
    }

    pub fn open_kv_only(
        db_paths: StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            true,
            internal_indexer_db,
            HotStateConfig {
                delete_on_restart: !readonly,
                ..Default::default()
            },
        )
    }

    pub fn open_dbs(
        db_paths: &StorageDirPaths,
        rocksdb_configs: RocksdbConfigs,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        max_num_nodes_per_lru_cache_shard: usize,
        hot_state_config: HotStateConfig,
    ) -> Result<(LedgerDb, StateMerkleDb, StateMerkleDb, StateKvDb, StateKvDb)> {
        std::thread::scope(|s| {
            let ledger_handle = s.spawn(|| {
                LedgerDb::new(
                    db_paths.ledger_db_root_path(),
                    rocksdb_configs.ledger_db_config,
                    env,
                    block_cache,
                    readonly,
                )
            });
            let hot_state_kv_handle = s.spawn(|| {
                StateKvDb::new(
                    db_paths,
                    rocksdb_configs.state_kv_db_config,
                    env,
                    block_cache,
                    readonly,
                    /* is_hot = */ true,
                    hot_state_config.delete_on_restart,
                )
            });
            let state_kv_handle = s.spawn(|| {
                StateKvDb::new(
                    db_paths,
                    rocksdb_configs.state_kv_db_config,
                    env,
                    block_cache,
                    readonly,
                    /* is_hot = */ false,
                    /* delete_on_restart = */ false,
                )
            });
            let hot_state_merkle_handle = s.spawn(|| {
                StateMerkleDb::new(
                    db_paths,
                    rocksdb_configs.state_merkle_db_config,
                    env,
                    block_cache,
                    readonly,
                    max_num_nodes_per_lru_cache_shard,
                    /* is_hot = */ true,
                    hot_state_config.delete_on_restart,
                )
            });
            let state_merkle_handle = s.spawn(|| {
                StateMerkleDb::new(
                    db_paths,
                    rocksdb_configs.state_merkle_db_config,
                    env,
                    block_cache,
                    readonly,
                    max_num_nodes_per_lru_cache_shard,
                    /* is_hot = */ false,
                    /* delete_on_restart = */ false,
                )
            });

            Ok((
                ledger_handle
                    .join()
                    .expect("Ledger db open thread panicked")?,
                hot_state_merkle_handle
                    .join()
                    .expect("Hot state merkle db open thread panicked")?,
                state_merkle_handle
                    .join()
                    .expect("State merkle db open thread panicked")?,
                hot_state_kv_handle
                    .join()
                    .expect("Hot state kv db open thread panicked")?,
                state_kv_handle
                    .join()
                    .expect("State kv db open thread panicked")?,
            ))
        })
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
    pub fn create_checkpoint(db_path: impl AsRef<Path>, cp_path: impl AsRef<Path>) -> Result<()> {
        let start = Instant::now();

        info!("Creating checkpoint for AptosDB.");

        LedgerDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref())?;
        StateKvDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref(), /* is_hot = */ true)?;
        StateKvDb::create_checkpoint(
            db_path.as_ref(),
            cp_path.as_ref(),
            /* is_hot = */ false,
        )?;
        StateMerkleDb::create_checkpoint(
            db_path.as_ref(),
            cp_path.as_ref(),
            /* is_hot = */ true,
        )?;
        StateMerkleDb::create_checkpoint(
            db_path.as_ref(),
            cp_path.as_ref(),
            /* is_hot = */ false,
        )?;
        // Native-position DBs. The static `create_checkpoint` opens
        // the source DB from `db_path` (creating it if absent), then
        // checkpoints into `cp_path`. Deployments that never activated
        // native-position still produce empty position checkpoints —
        // matches state's always-create behavior.
        PositionDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref())?;
        PositionMerkleDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref())?;

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
