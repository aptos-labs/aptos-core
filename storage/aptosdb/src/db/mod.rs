// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup::backup_handler::BackupHandler, event_store::EventStore, ledger_db::LedgerDb,
    native_state_committer::NativeStateCommitter, native_state_store::NativeStateStore,
    position_db::PositionDb, position_merkle_db::PositionMerkleDb,
    position_state_store::PositionStateStore, pruner::LedgerPrunerManager,
    rocksdb_property_reporter::RocksdbPropertyReporter, state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb, state_store::StateStore, transaction_store::TransactionStore,
};
use aptos_config::config::{HotStateConfig, PrunerConfig, RocksdbConfigs, StorageDirPaths};
use aptos_crypto::HashValue;
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
    /// Native-mirror subsystem. All handles are `None` until the
    /// feature flag activates and the open path initializes them.
    /// `native_state` is the per-user store; `position_db` /
    /// `position_merkle_db` back the durable Position CF + JMT.
    #[allow(dead_code)]
    pub(crate) position_db: Option<Arc<PositionDb>>,
    #[allow(dead_code)]
    pub(crate) position_merkle_db: Option<Arc<PositionMerkleDb>>,
    #[allow(dead_code)]
    pub(crate) native_state: Option<Arc<NativeStateStore>>,
    /// Owner of the async commit pipeline for `position_merkle_db`.
    /// Mirrors the role of `state_store` for main state: holds the
    /// shared `current_state` mutex + the buffered state itself.
    /// Constructed by `init_native_position`.
    pub(crate) position_state_store: Option<Arc<PositionStateStore>>,
}

/// Snapshot of the native-mirror storage handles attached to an
/// `AptosDB`. Any that are `None` indicate the subsystem hasn't been
/// initialized on this node.
pub struct NativePositionHandles {
    pub position_db: Option<Arc<PositionDb>>,
    pub position_merkle_db: Option<Arc<PositionMerkleDb>>,
    pub native_state: Option<Arc<NativeStateStore>>,
}

impl AptosDB {
    pub fn native_position_handles(&self) -> NativePositionHandles {
        NativePositionHandles {
            position_db: self.position_db.clone(),
            position_merkle_db: self.position_merkle_db.clone(),
            native_state: self.native_state.clone(),
        }
    }

    /// Build a [`PositionPruner`] over `position_db`. Returns `None`
    /// if the subsystem hasn't been initialized.
    pub fn position_pruner(&self) -> Option<crate::position_pruner::PositionPruner> {
        let position_db = self.position_db.clone()?;
        Some(crate::position_pruner::PositionPruner::new(position_db))
    }

    /// Build a ready-to-use [`NativeStateCommitter`] from the attached
    /// handles. Returns `None` if the subsystem hasn't been
    /// initialized.
    pub fn native_state_committer(&self) -> Option<NativeStateCommitter> {
        let position_db = self.position_db.clone()?;
        let store = self.native_state.clone()?;
        let mut c = NativeStateCommitter::new(position_db, store);
        if let Some(merkle) = self.position_merkle_db.clone() {
            c = c.with_position_merkle_db(merkle);
        }
        Some(c)
    }

    /// Opt-in initialization of the native-position subsystem.
    ///
    /// Opens `position_db` / `position_merkle_db` at the given paths
    /// using the column-family options derived from `rocksdb_config`
    /// (block cache, write buffer, compression — all the standard
    /// tuning). Builds an empty `NativeStateStore` and attaches the
    /// handles to the running `AptosDB`. Callers typically invoke
    /// this from the node-open path once the `NATIVE_POSITION` feature
    /// flag has activated on-chain.
    ///
    /// Idempotent in the sense that a second call aborts with
    /// `AlreadyExists` rather than silently dropping the previously
    /// installed handles + in-memory store. Startup-time population
    /// of the in-memory store from `position_db` (the cold-load scan)
    /// is a subsequent step the caller performs after this
    /// initializer returns.
    pub fn init_native_position(
        &mut self,
        position_db_path: &std::path::Path,
        position_merkle_db_path: &std::path::Path,
        rocksdb_config: aptos_config::config::RocksdbConfig,
        readonly: bool,
    ) -> Result<()> {
        if self.position_db.is_some()
            || self.position_merkle_db.is_some()
            || self.native_state.is_some()
        {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let env = aptos_schemadb::Env::new()
            .map_err(|e| AptosDbError::Other(format!("failed to create RocksDB env: {e}")))?;

        // Open the value DB. `PositionDb::new` does the per-shard +
        // metadata RocksDB open with production CF tuning — same shape
        // as `StateKvDb::new`.
        let position_db = crate::position_db::PositionDb::new(
            position_db_path,
            rocksdb_config,
            Some(&env),
            None,
            readonly,
        )?;

        // Truncation-on-startup: if a previous run crashed mid-commit,
        // the per-shard rows may be ahead of `PositionCommitProgress`.
        // Roll back any rows at versions strictly greater than the
        // last fully-committed version. Mirrors `state_kv_db`'s
        // open-time truncation. Skipped on readonly mounts.
        if !readonly
            && let Some(progress) =
                crate::utils::truncation_helper::get_position_commit_progress(&position_db)?
        {
            crate::utils::truncation_helper::truncate_position_db_shards(&position_db, progress)?;
        }

        // Open the merkle DB. Same shape as `StateMerkleDb::new`.
        let merkle_db = Arc::new(PositionMerkleDb::new(
            position_merkle_db_path,
            rocksdb_config,
            Some(&env),
            None,
            readonly,
            /* max_nodes_per_lru_cache_shard — caches off for now */ 0,
        )?);

        self.position_db = Some(Arc::new(position_db));
        self.position_merkle_db = Some(merkle_db.clone());
        let store = Arc::new(NativeStateStore::empty());
        self.native_state = Some(store.clone());
        crate::native_state_reader::install_global_reader(Arc::new(
            crate::native_state_reader::InMemoryNativeStateReader::new(store),
        ));
        // Publish pruner handle too. Built from the just-attached
        // DB via .clone() of the Arc.
        let pos_pruner = Arc::new(crate::position_pruner::PositionPruner::new(
            self.position_db.clone().unwrap(),
        ));
        crate::native_state_reader::install_global_pruners(pos_pruner);

        // Re-bind LedgerPrunerManager so its sub-pruners include the
        // native value-CF pruner now that the DB is open. Before
        // this point, the manager runs without it.
        self.ledger_pruner
            .attach_native_pruners(self.position_db.clone().unwrap());

        // Initialize the async commit pipeline. Seeded with the
        // empty-tree placeholder root. The first `update` call
        // advances the scratchpad SMT against this base. If the DB
        // already holds prior position state, the SMT's `ProofRead`
        // impl fetches structure on demand from `position_merkle_db`
        // (no eager cold-load of the SMT needed — the existing
        // `populate_native_state_from_db` covers the in-memory value
        // mirror separately).
        if !readonly {
            let last_snapshot = crate::position_buffered_state::new_empty_position_state();
            let store = PositionStateStore::new_at_snapshot(
                merkle_db,
                Arc::clone(&self.ledger_db),
                last_snapshot,
            );
            self.position_state_store = Some(Arc::new(store));
        }

        info!(
            position_db_path = %position_db_path.display(),
            position_merkle_db_path = %position_merkle_db_path.display(),
            num_shards = crate::position_db::NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }

    /// Build a [`crate::native_state_reader::InMemoryNativeStateReader`]
    /// that exposes the validator-side reader trait over the attached
    /// in-memory store. Returns `None` when the native-position
    /// subsystem has not been initialized.
    pub fn native_state_reader(
        &self,
    ) -> Option<crate::native_state_reader::InMemoryNativeStateReader> {
        let store = self.native_state.clone()?;
        Some(crate::native_state_reader::InMemoryNativeStateReader::new(
            store,
        ))
    }

    /// Populate the in-memory position store from `position_db` at
    /// version `<= version`. Typically called by the node-open path
    /// right after `init_native_position` returns, before the node
    /// starts executing blocks. `parallel = true` fans the scan out
    /// across the 16 shards via rayon; `false` walks them
    /// sequentially (useful when the caller wants deterministic
    /// resource use during catchup).
    ///
    /// Returns the number of positions loaded. Observes the total
    /// wall-clock scan duration in `aptos_position_cold_load_seconds`.
    pub fn load_native_position_from_disk(
        &self,
        version: Version,
        parallel: bool,
    ) -> Result<usize> {
        use std::time::Instant;

        let position_db = self
            .position_db
            .as_ref()
            .ok_or_else(|| AptosDbError::Other("position_db not initialized".to_string()))?;
        let position_merkle_db = self
            .position_merkle_db
            .as_ref()
            .ok_or_else(|| AptosDbError::Other("position_merkle_db not initialized".to_string()))?;
        let store = self
            .native_state
            .as_ref()
            .ok_or_else(|| AptosDbError::Other("native_state not initialized".to_string()))?;

        let _ = parallel; // Reserved for a future parallel JMT walk.
        let t0 = Instant::now();
        // Lifecycle metadata (exchange-id allocations + deny-list)
        // lives in the Move ExchangeRegistry resource and is
        // re-hydrated automatically as part of the main state-kv
        // load — nothing to do here.
        //
        // The value CF is hash-keyed, so we can't enumerate
        // `(exchange, account, market)` from value rows alone.
        // Walk the JMT at `version` to enumerate live leaves
        // (each carries the original `StateKey`), then fetch the
        // value by hash.
        use aptos_crypto::hash::CryptoHash;
        let mut rows: Vec<(
            aptos_types::state_store::state_key::StateKey,
            aptos_types::state_store::state_value::StateValue,
        )> = Vec::new();
        for leaf in position_merkle_db.iter_active_leaves(version)? {
            let (state_key, _key_hash) = leaf?;
            let value = position_db
                .get_position_value(state_key.hash(), version)?
                .ok_or_else(|| {
                    AptosDbError::Other(format!(
                        "cold-load: JMT leaf at version {version} has no matching \
                         position_value row for state_key_hash {}",
                        state_key.hash()
                    ))
                })?;
            rows.push((state_key, value));
        }
        let n_positions = store
            .populate_from_rows(rows)
            .map_err(|e| AptosDbError::Other(format!("populate_from_rows: {e}")))?;
        let elapsed = t0.elapsed();
        crate::position_metrics::POSITION_COLD_LOAD_SECONDS.observe(elapsed.as_secs_f64());
        info!(
            version = version,
            n_positions = n_positions,
            elapsed_ms = elapsed.as_millis() as u64,
            "Native-position cold-load complete."
        );
        Ok(n_positions)
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
            hot_state_config.persist_hotness_in_write_set,
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
