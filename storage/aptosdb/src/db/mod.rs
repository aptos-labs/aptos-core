// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup::backup_handler::BackupHandler,
    event_store::EventStore,
    ledger_db::LedgerDb,
    native_state_committer::NativeStateCommitter,
    position_buffered_state::new_empty_position_state,
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_state_store::PositionStateStore,
    pruner::LedgerPrunerManager,
    rocksdb_property_reporter::RocksdbPropertyReporter,
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    state_store::StateStore,
    transaction_store::TransactionStore,
    utils::truncation_helper::{
        get_position_commit_progress, get_position_merkle_commit_progress,
        truncate_position_db_shards, truncate_position_merkle_db_shards,
    },
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

/// Flip to `true` once order/collateral land.
pub(crate) const ENABLE_NATIVE_POSITION: bool = false;

/// This holds a handle to the underlying DB responsible for physical storage and provides APIs for
/// access to the core Aptos data structures.
pub struct AptosDB {
    pub(crate) ledger_db: Arc<LedgerDb>,
    pub(crate) hot_state_kv_db: Option<Arc<StateKvDb>>,
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

pub struct PositionBundle {
    pub kv_db: Arc<PositionDb>,
    pub merkle_db: Arc<PositionMerkleDb>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<PositionStateStore>>,
}

impl AptosDB {
    pub fn position(&self) -> Option<&Arc<PositionBundle>> {
        self.position.as_ref()
    }

    pub fn native_state_committer(&self) -> Option<NativeStateCommitter> {
        let bundle = self.position.as_ref()?;
        Some(NativeStateCommitter::new(bundle.kv_db.clone()))
    }

    /// Called automatically from `open_internal` when
    /// `ENABLE_NATIVE_POSITION` is `true`.
    pub fn init_native_position(
        &mut self,
        db_paths: &StorageDirPaths,
        rocksdb_config: aptos_config::config::RocksdbConfig,
        readonly: bool,
    ) -> Result<()> {
        if self.position.is_some() {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let env = aptos_schemadb::Env::new()
            .map_err(|e| AptosDbError::Other(format!("failed to create RocksDB env: {e}")))?;

        let position_db = PositionDb::new(db_paths, rocksdb_config, Some(&env), None, readonly)?;
        if !readonly && let Some(progress) = get_position_commit_progress(&position_db)? {
            truncate_position_db_shards(&position_db, progress)?;
        }

        let merkle_db = PositionMerkleDb::new(
            db_paths,
            rocksdb_config,
            Some(&env),
            None,
            readonly,
            /* max_nodes_per_lru_cache_shard */ 0,
        )?;
        if !readonly && let Some(progress) = get_position_merkle_commit_progress(&merkle_db)? {
            truncate_position_merkle_db_shards(&merkle_db, progress)?;
        }
        let kv_db = Arc::new(position_db);
        let merkle_db = Arc::new(merkle_db);

        let state_store = if readonly {
            None
        } else {
            let last_snapshot = new_empty_position_state();
            Some(Arc::new(PositionStateStore::new_at_snapshot(
                Arc::clone(&merkle_db),
                Arc::clone(&self.ledger_db),
                last_snapshot,
            )))
        };

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            state_store,
        }));

        info!(
            num_shards = NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }
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
            HotStateConfig::default(),
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
    ) -> Result<(
        LedgerDb,
        Option<StateMerkleDb>,
        StateMerkleDb,
        Option<StateKvDb>,
        StateKvDb,
    )> {
        let ledger_db = LedgerDb::new(
            db_paths.ledger_db_root_path(),
            rocksdb_configs.ledger_db_config,
            env,
            block_cache,
            readonly,
        )?;
        let hot_state_kv_db = if !readonly {
            Some(StateKvDb::new(
                db_paths,
                rocksdb_configs.state_kv_db_config,
                env,
                block_cache,
                readonly,
                /* is_hot = */ true,
                hot_state_config.delete_on_restart,
            )?)
        } else {
            None
        };
        let state_kv_db = StateKvDb::new(
            db_paths,
            rocksdb_configs.state_kv_db_config,
            env,
            block_cache,
            readonly,
            /* is_hot = */ false,
            /* delete_on_restart = */ false,
        )?;
        let hot_state_merkle_db = if !readonly {
            Some(StateMerkleDb::new(
                db_paths,
                rocksdb_configs.state_merkle_db_config,
                env,
                block_cache,
                readonly,
                max_num_nodes_per_lru_cache_shard,
                /* is_hot = */ true,
                hot_state_config.delete_on_restart,
            )?)
        } else {
            None
        };
        let state_merkle_db = StateMerkleDb::new(
            db_paths,
            rocksdb_configs.state_merkle_db_config,
            env,
            block_cache,
            readonly,
            max_num_nodes_per_lru_cache_shard,
            /* is_hot = */ false,
            /* delete_on_restart = */ false,
        )?;

        Ok((
            ledger_db,
            hot_state_merkle_db,
            state_merkle_db,
            hot_state_kv_db,
            state_kv_db,
        ))
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
