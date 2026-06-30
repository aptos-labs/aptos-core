// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_state_committer::NativeStateCommitter,
    position_buffered_state::{
        new_empty_position_state, position_state_at_version, PositionLedgerStateWithSummary,
        PositionPersistedState, PositionProofReader, PositionSlot,
    },
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_pruner::PositionPruner,
    position_state_store::PositionStateStore,
    utils::truncation_helper::{
        get_position_commit_progress, get_position_merkle_commit_progress,
        truncate_position_db_shards, truncate_position_merkle_db,
    },
};
use aptos_config::config::{
    LedgerPrunerConfig, RocksdbConfig, StateMerklePrunerConfig, StorageDirPaths,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{state_store::state_value::StateValue, transaction::Version};
use std::{collections::HashMap, sync::Arc};

pub struct PositionBundle {
    pub kv_db: Arc<PositionDb>,
    pub merkle_db: Arc<PositionMerkleDb>,
    /// Pruner managers (value + merkle), the analog of main state's
    /// `StatePruner`. `None` in readonly mode. Held as `Arc` so the
    /// position merkle batch committer shares it; the value pruner is
    /// driven from `commit_native_position`, the merkle pruners from the
    /// committer, and all are re-activated on restart from `open_internal`.
    pub(crate) position_pruner: Option<Arc<PositionPruner>>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<PositionStateStore>>,
    /// Latest persisted in-memory snapshot — the base the in-memory
    /// chain rebases onto each chunk (SMT freeze base + proof
    /// version). Advanced by the merkle batch committer as snapshots
    /// persist, so the proof base tracks the JMT forward and the
    /// in-memory tree sheds nodes below it. `None` in readonly mode.
    pub(crate) persisted: Option<PositionPersistedState>,
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
    /// `ENABLE_TRADING_NATIVE` is `true`. Shares `env` and
    /// `block_cache` with the main AptosDB so RocksDB background
    /// threads and the block cache stay singleton.
    pub fn init_native_position(
        &mut self,
        db_paths: &StorageDirPaths,
        kv_config: RocksdbConfig,
        merkle_config: RocksdbConfig,
        value_pruner_config: LedgerPrunerConfig,
        state_merkle_pruner_config: StateMerklePrunerConfig,
        epoch_snapshot_pruner_config: StateMerklePrunerConfig,
        env: &Env,
        block_cache: &Cache,
        readonly: bool,
    ) -> Result<()> {
        if self.position.is_some() {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let position_db =
            PositionDb::new(db_paths, kv_config, Some(env), Some(block_cache), readonly)?;
        let merkle_db = PositionMerkleDb::new(
            db_paths,
            merkle_config,
            Some(env),
            Some(block_cache),
            readonly,
            /* max_nodes_per_lru_cache_shard */ 0,
        )?;

        // Mirror `StateStore::sync_commit_progress`: align both
        // position DBs with the ledger's `OverallCommitProgress`
        // (truncating ahead-of-chain rows from a crash) and find the
        // latest JMT snapshot at or before that point.
        let merkle_progress = if readonly {
            None
        } else {
            self.sync_position_commit_progress(&position_db, &merkle_db)?
        };

        let kv_db = Arc::new(position_db);
        let merkle_db = Arc::new(merkle_db);

        // Pruner managers (value + merkle), grouped like main state's
        // `StatePruner`. Shared with the merkle batch committer via `Arc`.
        let position_pruner = if readonly {
            None
        } else {
            Some(Arc::new(PositionPruner::new(
                Arc::clone(&kv_db),
                Arc::clone(&merkle_db),
                value_pruner_config,
                state_merkle_pruner_config,
                epoch_snapshot_pruner_config,
            )))
        };

        let (state_store, persisted) = if readonly {
            (None, None)
        } else {
            let last_snapshot = match merkle_progress {
                Some(version) => {
                    let root_hash = merkle_db.get_root_hash(version)?;
                    position_state_at_version(version, root_hash)
                },
                None => new_empty_position_state(),
            };
            // Seed the persisted base with the exact snapshot used for
            // `current_state` so the first rebase freezes against an
            // in-family ancestor (the chain descends from this seed).
            let persisted = PositionPersistedState::new(last_snapshot.clone());
            let store = Arc::new(PositionStateStore::new_at_snapshot(
                Arc::clone(&merkle_db),
                Arc::clone(&self.ledger_db),
                last_snapshot,
                Arc::clone(
                    position_pruner
                        .as_ref()
                        .expect("position_pruner present in non-readonly mode"),
                ),
                persisted.clone(),
            ));
            (Some(store), Some(persisted))
        };

        // Replay write sets between the JMT snapshot and the chain
        // tip so the in-memory pipeline + the JMT catch up to
        // `OverallCommitProgress`.
        if let Some(store) = state_store.as_ref()
            && let Some(v_overall) = self.ledger_db.metadata_db().get_synced_version()?
        {
            let snapshot_next_version = merkle_progress.map_or(0, |v| v + 1);
            if snapshot_next_version <= v_overall {
                self.replay_position_after_snapshot(
                    store,
                    &merkle_db,
                    snapshot_next_version,
                    v_overall + 1,
                )?;
            }
        }

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            position_pruner,
            state_store,
            persisted,
        }));

        info!(
            num_shards = NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }

    /// Align position DB progress with the chain. Truncates `kv_db`
    /// down to `OverallCommitProgress` if it ran ahead (crash between
    /// the position commit and the ledger's commit-progress write),
    /// and returns the merkle DB's latest snapshot version after
    /// truncating it to its own progress. The merkle DB should never
    /// exceed `OverallCommitProgress` in normal operation.
    fn sync_position_commit_progress(
        &self,
        position_db: &PositionDb,
        merkle_db: &PositionMerkleDb,
    ) -> Result<Option<Version>> {
        let v_overall = self.ledger_db.metadata_db().get_synced_version()?;

        if let Some(v_kv) = get_position_commit_progress(position_db)? {
            let target = v_overall.map_or(0, |v| std::cmp::min(v_kv, v));
            if v_kv != target {
                info!(
                    v_kv = v_kv,
                    v_overall = ?v_overall,
                    target = target,
                    "Truncating position_db down to chain's OverallCommitProgress."
                );
            }
            truncate_position_db_shards(position_db, target)?;
        }

        let Some(v_merkle) = get_position_merkle_commit_progress(merkle_db)? else {
            return Ok(None);
        };

        // `pre_commit_ledger()` runs `commit_native_position()` — which can
        // advance the position merkle snapshot — before `commit_ledger()`
        // records `OverallCommitProgress`. A crash in that window leaves the
        // merkle DB ahead of the chain, so truncate down to the latest snapshot
        // at or before the chain tip rather than panicking (matches main
        // state's restart handling).
        let target = match v_overall {
            Some(v) if v < v_merkle => merkle_db.latest_snapshot_version_at_or_before(v)?,
            _ => Some(v_merkle),
        };
        let target = target.ok_or_else(|| {
            AptosDbError::Other(format!(
                "position_merkle_db has no snapshot at or before chain version {v_overall:?}; \
                 only an uncommitted snapshot at {v_merkle} exists"
            ))
        })?;
        truncate_position_merkle_db(merkle_db, target)?;
        Ok(Some(target))
    }

    /// Replay `WriteSet`s in `[snapshot_next_version, num_transactions)`
    /// — the gap between the persisted JMT snapshot and the chain
    /// tip. Coalesces latest-wins-per-key, extends in one shot, and
    /// sync-commits the resulting snapshot before returning.
    fn replay_position_after_snapshot(
        &self,
        store: &PositionStateStore,
        merkle_db: &Arc<PositionMerkleDb>,
        snapshot_next_version: Version,
        num_transactions: u64,
    ) -> Result<()> {
        info!(
            snapshot_next_version = snapshot_next_version,
            num_transactions = num_transactions,
            "Replaying position write sets to catch up the in-memory pipeline."
        );

        let write_sets = self
            .ledger_db
            .write_set_db()
            .get_write_sets(snapshot_next_version, num_transactions)?;

        let mut pending_leaf_updates: HashMap<HashValue, PositionSlot> = HashMap::new();
        for write_set in &write_sets {
            for (key, op) in write_set.native_position_iter() {
                let maybe_value = op.as_write_op().as_state_value_opt().cloned();
                let value_hash = maybe_value.as_ref().map(StateValue::hash);
                pending_leaf_updates.insert(key.hash(), PositionSlot {
                    state_key: key.clone(),
                    value_hash,
                    value: None,
                });
            }
        }

        if pending_leaf_updates.is_empty() {
            return Ok(());
        }

        let state_lock = store.current_state();
        let pipeline_latest = state_lock.lock().latest().clone();
        let snapshot_version = pipeline_latest.version();

        let target_version = num_transactions - 1;
        let updates: Vec<_> = pending_leaf_updates.into_iter().collect();
        let proof_reader = PositionProofReader {
            merkle_db: Arc::clone(merkle_db),
            version: snapshot_version,
        };
        // At replay start the persisted base equals the seed, which is
        // `pipeline_latest` itself — freeze against it.
        let base_summary = pipeline_latest.summary().clone();
        let new_latest =
            pipeline_latest.extend(target_version, updates, &base_summary, &proof_reader)?;

        // Treat the target as a checkpoint so the buffered_state
        // sync-commits the JMT snapshot before we return.
        let new_state = PositionLedgerStateWithSummary::from_latest_and_last_checkpoint(
            new_latest.clone(),
            new_latest,
        );
        let mut bufstate = store.buffered_state_locked();
        bufstate.update(
            new_state,
            (),
            write_sets.len(),
            /* sync_commit = */ true,
        )?;
        Ok(())
    }
}
