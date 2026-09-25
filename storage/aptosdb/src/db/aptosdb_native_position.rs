// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_state_committer::NativeStateCommitter,
    native_state_reader::{install_global_reader, InMemoryNativeStateReader},
    native_state_store::{position_key_of, PositionBase, PositionOverlay, PositionWrites},
    position_buffered_state::{
        new_empty_position_state, position_state_at_version, PositionLedgerStateWithSummary,
        PositionPersistedState, PositionProofReader, PositionSlot,
        MAX_POSITION_WRITE_SETS_AFTER_SNAPSHOT,
    },
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_pruner::PositionPruner,
    position_state_store::PositionStateStore,
    pruner::PrunerManager,
    utils::truncation_helper::{
        get_position_commit_progress, truncate_position_db_shards, truncate_position_merkle_db,
    },
};
use aptos_config::config::{
    LedgerPrunerConfig, RocksdbConfig, StateMerklePrunerConfig, StorageDirPaths,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
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
    /// Complete account-grouped snapshot at the last folded version.
    /// Resolves overlay misses, which is what lets `positions` be
    /// a bounded delta rather than a chain pinned to a family root.
    /// Advanced by `advance_position_base`, which the executor calls
    /// while holding its execution lock so the base cannot move under a
    /// block's reads.
    pub(crate) position_base: Arc<PositionBase>,
    /// Tip overlay over `position_base`, extended once
    /// `position_db.commit(...)` succeeds. In readonly mode it stays at
    /// cold-load. External callers reach this via [`NativeStateReader`]
    /// (see `native_state_reader()`), not through the bundle directly.
    ///
    /// That commit runs in `pre_commit_ledger`, so this is the
    /// pre-committed tip, not the proven one. Risk and ADL want the tip:
    /// they decide about the block being built, not the last certified
    /// one. The cost is that a pre-committed block which loses its fork is
    /// briefly visible here.
    pub(crate) positions: Arc<Mutex<PositionOverlay>>,
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

    pub fn native_state_reader(&self) -> Option<InMemoryNativeStateReader> {
        let bundle = self.position.as_ref()?;
        Some(InMemoryNativeStateReader::new(Arc::clone(
            &bundle.positions,
        )))
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

        // Cold-load: stream the durable JMT snapshot into `PositionBase`,
        // decoding row by row so the live set is never materialized twice.
        // The gap `[snapshot_version + 1, chain_tip]` left by a crash
        // between JMT-snapshot and chain commit is closed by
        // `replay_position_after_snapshot`.
        let position_base = match merkle_progress {
            Some(snapshot_version) => {
                let iter = merkle_db.iter_active_leaves_with_values(
                    Arc::clone(&kv_db),
                    snapshot_version,
                    0,
                )?;
                let base = PositionBase::new_from_rows(Some(snapshot_version), "position", iter)?;
                info!(
                    snapshot_version = snapshot_version,
                    n_accounts = base.read(|view| view.num_accounts()),
                    "Native-position cold-load complete."
                );
                base
            },
            None => PositionBase::new_empty("position"),
        };
        let position_base = Arc::new(position_base);
        let positions = Arc::new(Mutex::new(PositionOverlay::new_at_base(Arc::clone(
            &position_base,
        ))));
        install_global_reader(Arc::new(InMemoryNativeStateReader::new(Arc::clone(
            &positions,
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
            // With no snapshot, start where the write sets still exist
            // rather than at 0: a fast-synced node has no history below its
            // target, and a pruned one cannot replay below the pruner.
            let snapshot_next_version = merkle_progress
                .map_or_else(|| self.ledger_pruner.get_min_readable_version(), |v| v + 1);
            if snapshot_next_version <= v_overall {
                self.replay_position_after_snapshot(
                    store,
                    &merkle_db,
                    snapshot_next_version,
                    v_overall + 1,
                    &positions,
                )?;
            }
        }

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            position_pruner,
            position_base,
            positions,
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

    /// Rebuild the position subsystem against the snapshot a fast sync just
    /// restored at `version`. The resident index, the overlay, and the JMT
    /// pipeline's baseline were all built at open time from a then-empty
    /// database that the restore has since replaced underneath them.
    pub(crate) fn reset_position_after_fast_sync(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<()> {
        let Some(bundle) = self.position.as_ref() else {
            return Ok(());
        };

        // A chain with the root feature on but no positions yet commits the
        // empty-tree placeholder. The snapshot stage then streams no values
        // and writes no JMT node, so there is nothing to load — but the
        // baseline still has to move to `version`.
        if expected_root_hash == *SPARSE_MERKLE_PLACEHOLDER_HASH {
            bundle
                .position_base
                .reset_from_rows(Some(version), "position", std::iter::empty())?;
        } else {
            ensure!(
                bundle
                    .merkle_db
                    .latest_snapshot_version_at_or_before(version)?
                    == Some(version),
                "Fast sync restored no position JMT snapshot at version {version}, but the target \
                 transaction info commits position state root {expected_root_hash:?}.",
            );
            let root_hash = bundle.merkle_db.get_root_hash(version)?;
            ensure!(
                root_hash == expected_root_hash,
                "Restored position root {root_hash:?} at version {version} does not match the \
                 proved root {expected_root_hash:?}.",
            );
            let rows = bundle.merkle_db.iter_active_leaves_with_values(
                Arc::clone(&bundle.kv_db),
                version,
                0,
            )?;
            bundle
                .position_base
                .reset_from_rows(Some(version), "position", rows)?;
        }
        // The old overlay sat on the pre-restore layer family and is
        // unreachable now; every reader reaches this through the same `Arc`.
        *bundle.positions.lock() = PositionOverlay::new_at_base(Arc::clone(&bundle.position_base));

        if let (Some(store), Some(persisted)) =
            (bundle.state_store.as_ref(), bundle.persisted.as_ref())
        {
            store.reset_at_snapshot(
                Arc::clone(&bundle.merkle_db),
                Arc::clone(&self.ledger_db),
                position_state_at_version(version, expected_root_hash),
                Arc::clone(
                    bundle
                        .position_pruner
                        .as_ref()
                        .expect("position_pruner present whenever state_store is"),
                ),
                persisted.clone(),
            );
        }

        info!(
            version = version,
            n_accounts = bundle.position_base.read(|view| view.num_accounts()),
            "Position subsystem reset after fast sync."
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

        // Look for a tree root rather than reading `StateMerkleCommitProgress`,
        // which the fast-sync restore never writes (it lands nodes through
        // `commit_no_progress`), leaving a restored snapshot invisible. Main
        // state discovers its snapshot the same way. Bounding by
        // `OverallCommitProgress` also truncates a merkle DB left ahead of
        // the chain by a crash between `commit_native_position()` and
        // `commit_ledger()`.
        let ceiling = v_overall.unwrap_or(Version::MAX);
        let Some(target) = merkle_db.latest_snapshot_version_at_or_before(ceiling)? else {
            // `truncate_position_merkle_db` peels down to an older snapshot
            // and cannot empty the DB, so a lone snapshot above the chain
            // tip has to be refused rather than cleaned up. Cold-loading an
            // empty base beside it would leave the index and the durable
            // tree describing different versions.
            ensure!(
                merkle_db
                    .latest_snapshot_version_at_or_before(Version::MAX)?
                    .is_none(),
                "position_merkle_db has no snapshot at or before chain version \
                 {v_overall:?}, only one above it.",
            );
            return Ok(None);
        };
        truncate_position_merkle_db(merkle_db, target)?;
        Ok(Some(target))
    }

    /// Replay `WriteSet`s in `[snapshot_next_version, num_transactions)`
    /// — the gap between the persisted JMT snapshot and the chain
    /// tip. Coalesces latest-wins-per-key, extends the JMT pipeline
    /// state in one shot, and folds the per-account updates into the
    /// `PositionOverlay` before returning.
    fn replay_position_after_snapshot(
        &self,
        store: &PositionStateStore,
        merkle_db: &Arc<PositionMerkleDb>,
        snapshot_next_version: Version,
        num_transactions: u64,
        positions: &Arc<Mutex<PositionOverlay>>,
    ) -> Result<()> {
        info!(
            snapshot_next_version = snapshot_next_version,
            num_transactions = num_transactions,
            "Replaying position write sets to catch up the in-memory pipeline."
        );

        // Same guard main state applies via `MAX_WRITE_SETS_AFTER_SNAPSHOT`:
        // the replay path materializes every write set in the gap
        // into one `Vec`, so unbounded gaps are an OOM risk. A node
        // enabling trading-native before its first position snapshot
        // (merkle_progress == None ⇒ snapshot_next_version == 0)
        // against a long-running chain would otherwise pull the
        // entire history.
        let gap = num_transactions.saturating_sub(snapshot_next_version);
        ensure!(
            gap <= MAX_POSITION_WRITE_SETS_AFTER_SNAPSHOT,
            "Too many versions to replay after position snapshot. snapshot_next_version: {}, \
             num_transactions: {}, gap: {}, max: {}",
            snapshot_next_version,
            num_transactions,
            gap,
            MAX_POSITION_WRITE_SETS_AFTER_SNAPSHOT,
        );

        let write_sets = self
            .ledger_db
            .write_set_db()
            .get_write_sets(snapshot_next_version, num_transactions)?;

        let mut pending_leaf_updates: HashMap<HashValue, PositionSlot> = HashMap::new();
        let mut pending_position_writes = PositionWrites::new();
        for write_set in &write_sets {
            for (key, op) in write_set.native_position_iter() {
                let maybe_value = op.as_write_op().as_state_value_opt().cloned();
                let value_hash = maybe_value.as_ref().map(StateValue::hash);
                let position_key = position_key_of(key)?;
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
                pending_position_writes.push((position_key, typed));
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

        // Fold the replayed account-level updates into `PositionOverlay`;
        // this matches the durable JMT state we just extended to.
        {
            let mut user_pos = positions.lock();
            *user_pos = user_pos.extend(target_version, pending_position_writes);
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
