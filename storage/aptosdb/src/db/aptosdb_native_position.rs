// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::{aptosdb_writer::materialize_user_position_updates, AptosDB},
    native_state_committer::{PositionWrite, NativeStateCommitter},
    native_state_reader::{install_global_reader, InMemoryNativeStateReader},
    native_state_store::{decode_rows_to_user_position_states, UserPositionKey, UserPositions},
    position_buffered_state::{
        new_empty_position_state, position_state_at_version, PositionLedgerStateWithSummary,
        PositionProofReader, PositionSlot,
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
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{native_position::NativePosition, state_value::StateValue},
    transaction::Version,
};
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
    /// Per-account committed-state cache for validator-side scanners.
    /// Sits at the bundle level — separate from the JMT pipeline state
    /// — and is extended only after `position_db.commit(...)` succeeds.
    /// In readonly mode it stays at cold-load.
    pub user_positions: Arc<Mutex<UserPositions>>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<PositionStateStore>>,
    /// JMT version the in-memory MapLayer chain is rooted at — the
    /// version `PositionProofReader` queries. Stays at init time
    /// because the chain doesn't rebase as new snapshots are taken
    /// (the chain just grows). When chain-rebasing lands, update
    /// this alongside the rebase.
    pub(crate) snapshot_version: Option<Version>,
}

impl AptosDB {
    pub fn position(&self) -> Option<&Arc<PositionBundle>> {
        self.position.as_ref()
    }

    pub fn native_state_committer(&self) -> Option<NativeStateCommitter> {
        let bundle = self.position.as_ref()?;
        Some(NativeStateCommitter::new(bundle.kv_db.clone()))
    }

    pub fn native_state_reader(&self) -> Option<InMemoryNativeStateReader> {
        let bundle = self.position.as_ref()?;
        Some(InMemoryNativeStateReader::new(Arc::clone(&bundle.user_positions)))
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

        // Match `StateStore::sync_commit_progress`: align both
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

        // Cold-load: stream the durable JMT snapshot into a decoded
        // `(UserPositionKey, UserPositionState)` seed and hydrate the `UserPositions`
        // base layer. Memory is bounded by the live position set, not
        // by the on-disk snapshot. The gap
        // `[snapshot_version + 1, chain_tip]` from a crash between
        // JMT-snapshot and chain commit is closed by
        // `replay_position_after_snapshot`.
        let user_positions = match merkle_progress {
            Some(snapshot_version) => {
                let iter = merkle_db.iter_active_leaves_with_values(
                    Arc::clone(&kv_db),
                    snapshot_version,
                    0,
                )?;
                let seed = decode_rows_to_user_position_states(iter).map_err(|e| {
                    AptosDbError::Other(format!("decode_rows_to_user_position_states: {e}"))
                })?;
                info!(
                    snapshot_version = snapshot_version,
                    n_accounts = seed.len(),
                    "Native-position cold-load complete."
                );
                UserPositions::new_at_version(Some(snapshot_version), "position")
                    .with_seeded_base(snapshot_version, seed)
            },
            None => UserPositions::new_empty("position"),
        };
        let user_positions = Arc::new(Mutex::new(user_positions));
        install_global_reader(Arc::new(InMemoryNativeStateReader::new(Arc::clone(
            &user_positions,
        ))));

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

        let state_store = if readonly {
            None
        } else {
            let last_snapshot = match merkle_progress {
                Some(version) => {
                    let root_hash = merkle_db.get_root_hash(version)?;
                    position_state_at_version(version, root_hash)
                },
                None => new_empty_position_state(),
            };
            Some(Arc::new(PositionStateStore::new_at_snapshot(
                Arc::clone(&merkle_db),
                Arc::clone(&self.ledger_db),
                last_snapshot,
                Arc::clone(
                    position_pruner
                        .as_ref()
                        .expect("position_pruner present in non-readonly mode"),
                ),
            )))
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
                    &user_positions,
                )?;
            }
        }

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            position_pruner,
            user_positions,
            state_store,
            snapshot_version: merkle_progress,
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

        // If a crash interleaves between `bufstate.update(..)` (which
        // schedules the merkle snapshot commit on its own thread) and
        // `commit_ledger()` (which advances `OverallCommitProgress`),
        // restart can find `v_merkle > v_overall`. Truncate the merkle
        // db down to the latest snapshot at or before `v_overall`
        // instead of panicking — the layered chain on top of that
        // snapshot gets rebuilt by `replay_position_after_snapshot`.
        let progress = get_position_merkle_commit_progress(merkle_db)?;
        let recovered_progress = match (progress, v_overall) {
            (Some(v_merkle), Some(v)) if v_merkle > v => {
                let recoverable = merkle_db.latest_snapshot_version_at_or_before(v)?;
                info!(
                    v_merkle = v_merkle,
                    v_overall = v,
                    recoverable = ?recoverable,
                    "position_merkle_db is ahead of OverallCommitProgress; \
                     truncating to the latest recoverable snapshot."
                );
                match recoverable {
                    Some(target) => {
                        truncate_position_merkle_db(merkle_db, target)?;
                        Some(target)
                    },
                    None => {
                        // No snapshot at or before v_overall; the
                        // merkle db has nothing recoverable. Truncate
                        // away everything ahead of the chain and
                        // re-seed from genesis.
                        truncate_position_merkle_db(merkle_db, 0)?;
                        None
                    },
                }
            },
            (Some(v_merkle), _) => {
                truncate_position_merkle_db(merkle_db, v_merkle)?;
                Some(v_merkle)
            },
            (None, _) => None,
        };
        Ok(recovered_progress)
    }

    /// Replay `WriteSet`s in `[snapshot_next_version, num_transactions)`
    /// — the gap between the persisted JMT snapshot and the chain
    /// tip. Coalesces latest-wins-per-key, extends the JMT pipeline
    /// state in one shot, and folds the per-account updates into the
    /// `UserPositions` before returning.
    fn replay_position_after_snapshot(
        &self,
        store: &PositionStateStore,
        merkle_db: &Arc<PositionMerkleDb>,
        snapshot_next_version: Version,
        num_transactions: u64,
        user_positions: &Arc<Mutex<UserPositions>>,
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
        let mut pending_position_writes: Vec<PositionWrite> = Vec::new();
        for write_set in &write_sets {
            for (key, op) in write_set.native_position_iter() {
                let maybe_value = op.as_write_op().as_state_value_opt().cloned();
                let value_hash = maybe_value.as_ref().map(StateValue::hash);
                let (exchange, account, market) = match key.inner() {
                    aptos_types::state_store::state_key::inner::StateKeyInner::TradingNative(
                        aptos_types::state_store::state_key::inner::TradingNativeKey::Position {
                            exchange,
                            account,
                            market,
                        },
                    ) => (*exchange, *account, *market),
                    other => {
                        return Err(AptosDbError::Other(format!(
                            "non-Position native key in replay write set: {other:?}"
                        )));
                    },
                };
                let typed = match maybe_value.as_ref() {
                    Some(sv) => Some(NativePosition::deserialize(sv.bytes()).map_err(|e| {
                        AptosDbError::Other(format!(
                            "position value decode failed during replay: {e}"
                        ))
                    })?),
                    None => None,
                };
                pending_leaf_updates.insert(key.hash(), PositionSlot {
                    state_key: key.clone(),
                    value_hash,
                    value: None,
                });
                pending_position_writes.push(PositionWrite {
                    position_key: UserPositionKey { exchange, account },
                    market,
                    value: typed,
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
        let jmt_updates: Vec<_> = pending_leaf_updates.into_iter().collect();
        let proof_reader = PositionProofReader {
            merkle_db: Arc::clone(merkle_db),
            version: snapshot_version,
        };
        let new_latest = pipeline_latest.extend(target_version, jmt_updates, &proof_reader)?;

        // Fold the replayed account-level updates into `UserPositions`;
        // this matches the durable JMT state we just extended to.
        {
            let mut user_pos = user_positions.lock();
            let updates = materialize_user_position_updates(&user_pos, pending_position_writes);
            *user_pos = user_pos.extend(target_version, updates);
        }

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
