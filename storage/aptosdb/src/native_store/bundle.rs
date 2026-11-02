// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A native store attached to `AptosDB`: its value DB, JMT, pruners and
//! commit pipeline, with the open, commit, replay and fast-sync-reset steps
//! every store shares. Store-specific in-memory indexes sit beside it.

#![forbid(unsafe_code)]

use crate::{
    ledger_db::LedgerDb,
    native_state_committer::{new_sharded_kv_batches, InChunkPriorVersions, NativeStateCommitter},
    native_store::{
        buffered_state::{
            native_state_at_version, new_empty_native_state, NativeLedgerStateWithSummary,
            NativePersistedState, NativeProofReader, NativeStateWithSummary, PositionSlot,
            MAX_NATIVE_WRITE_SETS_AFTER_SNAPSHOT,
        },
        kv_db::NativeKvDb,
        merkle_db::NativeMerkleDb,
        pruner::NativeStorePruner,
        state_store::NativeStateStore,
        state_sync::NativeStoreReader,
        NativeStoreSpec,
    },
    pruner::PrunerManager,
    utils::truncation_helper::{
        get_native_commit_progress, truncate_native_kv_db_shards, truncate_native_merkle_db,
    },
};
use aptos_config::config::{
    LedgerPrunerConfig, RocksdbConfig, StateMerklePrunerConfig, StorageDirPaths,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_jellyfish_merkle::{
    node_type::{Node, NodeKey},
    TreeWriter,
};
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{chunk_to_commit::ChunkToCommit, db_ensure as ensure, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
    write_set::WriteSet,
};
use std::{collections::HashMap, sync::Arc};

/// RocksDB and pruner configs a native store opens with.
#[derive(Clone, Copy)]
pub(crate) struct NativeStoreConfigs {
    pub kv_config: RocksdbConfig,
    pub merkle_config: RocksdbConfig,
    pub value_pruner_config: LedgerPrunerConfig,
    pub state_merkle_pruner_config: StateMerklePrunerConfig,
    pub epoch_snapshot_pruner_config: StateMerklePrunerConfig,
}

/// Durable JMT rows at a snapshot, streamed for a cold-load or a rebuild.
pub(crate) type NativeSnapshotRows =
    Box<dyn Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync>;

pub(crate) struct NativeStoreBundle<T: NativeStoreSpec> {
    pub(crate) kv_db: Arc<NativeKvDb<T>>,
    pub(crate) merkle_db: Arc<NativeMerkleDb<T>>,
    /// Pruner managers (value + merkle), the analog of main state's
    /// `StatePruner`. `None` in readonly mode. Held as `Arc` so the merkle
    /// batch committer shares it; the value pruner is driven from
    /// `commit_chunk`, the merkle pruners from the committer, and all are
    /// re-activated on restart by `activate_pruners`.
    pub(crate) pruner: Option<Arc<NativeStorePruner<T>>>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<NativeStateStore>>,
    /// Latest persisted in-memory snapshot — the base the in-memory chain
    /// rebases onto each chunk (SMT freeze base + proof version). Advanced
    /// by the merkle batch committer as snapshots persist, so the proof
    /// base tracks the JMT forward and the in-memory tree sheds nodes below
    /// it. `None` in readonly mode.
    pub(crate) persisted: Option<NativePersistedState>,
}

impl<T: NativeStoreSpec> NativeStoreBundle<T> {
    /// Opens the store's DBs and aligns them with the ledger. Returns the
    /// JMT snapshot version the store opened at, which the caller
    /// cold-loads its index from and catches up past.
    pub(crate) fn open(
        db_paths: &StorageDirPaths,
        configs: &NativeStoreConfigs,
        env: &Env,
        block_cache: &Cache,
        ledger_db: &Arc<LedgerDb>,
        readonly: bool,
    ) -> Result<(Self, Option<Version>)> {
        let kv_db = NativeKvDb::<T>::new(
            db_paths,
            configs.kv_config,
            Some(env),
            Some(block_cache),
            readonly,
        )?;
        let merkle_db = NativeMerkleDb::<T>::new(
            db_paths,
            configs.merkle_config,
            Some(env),
            Some(block_cache),
            readonly,
            /* max_nodes_per_lru_cache_shard */ 0,
        )?;

        // Mirror `StateStore::sync_commit_progress`: align both DBs with
        // the ledger's `OverallCommitProgress` (truncating ahead-of-chain
        // rows from a crash) and find the latest JMT snapshot at or before
        // that point.
        let snapshot_version = if readonly {
            let ceiling = ledger_db
                .metadata_db()
                .get_synced_version()?
                .unwrap_or(Version::MAX);
            merkle_db.latest_snapshot_version_at_or_before(ceiling)?
        } else {
            Self::sync_commit_progress(ledger_db, &kv_db, &merkle_db)?
        };

        let kv_db = Arc::new(kv_db);
        let merkle_db = Arc::new(merkle_db);

        let pruner = (!readonly).then(|| {
            Arc::new(NativeStorePruner::new(
                Arc::clone(&kv_db),
                Arc::clone(&merkle_db),
                configs.value_pruner_config,
                configs.state_merkle_pruner_config,
                configs.epoch_snapshot_pruner_config,
            ))
        });

        let (state_store, persisted) = match pruner.as_ref() {
            None => (None, None),
            Some(pruner) => {
                let last_snapshot = match snapshot_version {
                    Some(version) => {
                        let root_hash = merkle_db.get_root_hash(version)?;
                        native_state_at_version::<T>(version, root_hash)
                    },
                    None => new_empty_native_state::<T>(),
                };
                // Seed the persisted base with the exact snapshot used for
                // `current_state` so the first rebase freezes against an
                // in-family ancestor (the chain descends from this seed).
                let persisted = NativePersistedState::new(last_snapshot.clone());
                let store = Arc::new(NativeStateStore::new_at_snapshot(
                    Arc::clone(&merkle_db),
                    Arc::clone(ledger_db),
                    last_snapshot,
                    Arc::clone(pruner),
                    persisted.clone(),
                ));
                (Some(store), Some(persisted))
            },
        };

        Ok((
            Self {
                kv_db,
                merkle_db,
                pruner,
                state_store,
                persisted,
            },
            snapshot_version,
        ))
    }

    /// Align DB progress with the chain. Truncates `kv_db` down to
    /// `OverallCommitProgress` if it ran ahead (crash between the native
    /// commit and the ledger's commit-progress write), and returns the
    /// merkle DB's latest snapshot version after truncating it to its own
    /// progress. The merkle DB should never exceed `OverallCommitProgress`
    /// in normal operation.
    fn sync_commit_progress(
        ledger_db: &LedgerDb,
        kv_db: &NativeKvDb<T>,
        merkle_db: &NativeMerkleDb<T>,
    ) -> Result<Option<Version>> {
        let v_overall = ledger_db.metadata_db().get_synced_version()?;

        if let Some(v_kv) = get_native_commit_progress(kv_db)? {
            let target = v_overall.map_or(0, |v| std::cmp::min(v_kv, v));
            if v_kv != target {
                info!(
                    store = T::NAME,
                    v_kv = v_kv,
                    v_overall = ?v_overall,
                    target = target,
                    "Truncating native kv db down to chain's OverallCommitProgress."
                );
            }
            truncate_native_kv_db_shards(kv_db, target)?;
        }

        // Look for a tree root rather than reading `StateMerkleCommitProgress`,
        // which the fast-sync restore never writes (it lands nodes through
        // `commit_no_progress`), leaving a restored snapshot invisible. Main
        // state discovers its snapshot the same way. Bounding by
        // `OverallCommitProgress` also truncates a merkle DB left ahead of
        // the chain by a crash between the native commit and
        // `commit_ledger()`.
        let ceiling = v_overall.unwrap_or(Version::MAX);
        let Some(target) = merkle_db.latest_snapshot_version_at_or_before(ceiling)? else {
            // `truncate_native_merkle_db` peels down to an older snapshot
            // and cannot empty the DB, so a lone snapshot above the chain
            // tip has to be refused rather than cleaned up. Cold-loading an
            // empty index beside it would leave the index and the durable
            // tree describing different versions.
            ensure!(
                merkle_db
                    .latest_snapshot_version_at_or_before(Version::MAX)?
                    .is_none(),
                "{} merkle db has no snapshot at or before chain version {v_overall:?}, only one \
                 above it.",
                T::NAME,
            );
            return Ok(None);
        };
        truncate_native_merkle_db(merkle_db, target)?;
        Ok(Some(target))
    }

    /// The JMT's live rows at `version`, decoded row by row by the caller so
    /// the live set is never materialized twice.
    pub(crate) fn snapshot_rows(&self, version: Version) -> Result<NativeSnapshotRows> {
        Ok(Box::new(self.merkle_db.iter_active_leaves_with_values(
            Arc::clone(&self.kv_db),
            version,
            0,
        )?))
    }

    /// The state-sync and backup producers.
    pub(crate) fn reader<'a>(&'a self, ledger_db: &'a LedgerDb) -> NativeStoreReader<'a, T> {
        NativeStoreReader {
            kv_db: &self.kv_db,
            merkle_db: &self.merkle_db,
            pruner: self.pruner.as_deref(),
            ledger_metadata_db: ledger_db.metadata_db(),
        }
    }

    /// Wakes the pruners after open; otherwise their workers idle at
    /// persisted progress until the next commit, even when committed
    /// versions are already ahead.
    pub(crate) fn activate_pruners(&self, synced_version: Option<Version>) {
        let Some(pruner) = self.pruner.as_ref() else {
            return;
        };
        if let Some(version) = synced_version {
            self.wake_value_pruner(version);
        }
        // Drive the merkle pruners to the persisted base version (set after
        // open's seed + replay).
        if let Some(version) = self
            .persisted
            .as_ref()
            .and_then(|persisted| persisted.get().version())
        {
            pruner
                .state_merkle_pruner
                .maybe_set_pruner_target_db_version(version);
            pruner
                .epoch_snapshot_pruner
                .maybe_set_pruner_target_db_version(version);
        }
    }

    /// Drives the value pruner to `version`, once the ledger commit there is
    /// durable. The merkle pruners are driven as snapshots persist.
    pub(crate) fn wake_value_pruner(&self, version: Version) {
        if let Some(pruner) = self.pruner.as_ref() {
            pruner
                .value_pruner
                .maybe_set_pruner_target_db_version(version);
        }
    }

    /// Write sets in `[snapshot_next_version, num_transactions)`, for
    /// replaying past a snapshot. Replays materialize the whole gap, so it
    /// is bounded like main state's `MAX_WRITE_SETS_AFTER_SNAPSHOT`. With
    /// no snapshot the gap starts at version 0, i.e. the entire history.
    pub(crate) fn write_sets_after_snapshot(
        ledger_db: &LedgerDb,
        snapshot_next_version: Version,
        num_transactions: Version,
    ) -> Result<Vec<WriteSet>> {
        let gap = num_transactions.saturating_sub(snapshot_next_version);
        ensure!(
            gap <= MAX_NATIVE_WRITE_SETS_AFTER_SNAPSHOT,
            "Too many versions to replay after {} snapshot. snapshot_next_version: {}, \
             num_transactions: {}, gap: {}, max: {}",
            T::NAME,
            snapshot_next_version,
            num_transactions,
            gap,
            MAX_NATIVE_WRITE_SETS_AFTER_SNAPSHOT,
        );
        ledger_db
            .write_set_db()
            .get_write_sets(snapshot_next_version, num_transactions)
    }

    /// Replays write sets between the JMT snapshot at `snapshot_version` and
    /// the chain tip so the in-memory pipeline and the JMT catch up to
    /// `OverallCommitProgress`. Returns the replayed write sets and the
    /// version they end at, for the caller's index; `None` when there is
    /// nothing to replay.
    ///
    /// No-op without a pipeline (readonly), or when nothing was ever
    /// committed to the store — a fast-synced node whose target carried no
    /// values for it.
    pub(crate) fn catch_up(
        &self,
        ledger_db: &LedgerDb,
        snapshot_version: Option<Version>,
    ) -> Result<Option<(Version, Vec<WriteSet>)>> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(None);
        };
        let Some(v_overall) = ledger_db.metadata_db().get_synced_version()? else {
            return Ok(None);
        };
        // No JMT snapshot means the index cold-loaded empty, so replay
        // covers every version; a higher floor would drop values last
        // written below it.
        let snapshot_next_version = snapshot_version.map_or(0, |v| v + 1);
        let Some(kv_progress) = get_native_commit_progress(&self.kv_db)? else {
            return Ok(None);
        };
        if snapshot_next_version > v_overall {
            return Ok(None);
        }
        let num_transactions = v_overall + 1;
        info!(
            store = T::NAME,
            snapshot_next_version = snapshot_next_version,
            num_transactions = num_transactions,
            "Replaying native write sets to catch up the in-memory pipeline."
        );

        let write_sets =
            Self::write_sets_after_snapshot(ledger_db, snapshot_next_version, num_transactions)?;

        // Versions above `kv_progress` were committed while native storage
        // was off, so their value rows are written here too: a JMT leaf
        // resolves its value as the latest row at or before the leaf's
        // version, and a missing row would resolve to an older value.
        let committer = NativeStateCommitter::new(Arc::clone(&self.kv_db));
        let mut sharded_kv_batches = new_sharded_kv_batches();
        let mut in_chunk_prior = InChunkPriorVersions::new();
        for (version, write_set) in (snapshot_next_version..).zip(&write_sets) {
            if version > kv_progress {
                committer.apply(
                    version,
                    T::write_set_ops(write_set),
                    &mut sharded_kv_batches,
                    &mut in_chunk_prior,
                )?;
            }
        }
        let target_version = v_overall;
        if target_version > kv_progress {
            self.kv_db
                .commit(target_version, None, sharded_kv_batches)?;
        }

        let leaf_updates = latest_leaf_updates::<T>(&write_sets);
        if leaf_updates.is_empty() {
            return Ok(None);
        }

        let state_lock = store.current_state();
        let pipeline_latest = state_lock.lock().latest().clone();
        let proof_reader = NativeProofReader {
            merkle_db: Arc::clone(&self.merkle_db),
            version: pipeline_latest.version(),
        };
        // At replay start the persisted base equals the seed, which is
        // `pipeline_latest` itself — freeze against it.
        let base = pipeline_latest.clone();
        let new_latest = pipeline_latest.extend(
            target_version,
            leaf_updates.into_iter().collect(),
            &base,
            &proof_reader,
        )?;

        // Treat the target as a checkpoint so the buffered state
        // sync-commits the JMT snapshot before we return.
        let new_state = NativeLedgerStateWithSummary::from_latest_and_last_checkpoint(
            new_latest.clone(),
            new_latest,
        );
        store.buffered_state_locked().update(
            new_state,
            (),
            write_sets.len(),
            /* sync_commit = */ true,
        )?;
        Ok(Some((target_version, write_sets)))
    }

    /// Re-seats the store on the snapshot a fast sync restored at
    /// `version`. Every in-memory structure was built at open time from a
    /// then-empty database the restore has since replaced underneath it.
    /// `reseat_index` rebuilds the caller's index from the restored rows,
    /// `None` for an empty tree.
    pub(crate) fn reset_after_fast_sync(
        &self,
        ledger_db: &Arc<LedgerDb>,
        version: Version,
        expected_root_hash: HashValue,
        reseat_index: impl FnOnce(Option<NativeSnapshotRows>) -> Result<()>,
    ) -> Result<()> {
        // A chain with the root feature on but no values yet commits the
        // empty-tree placeholder. The snapshot stage then streams no values
        // and writes no JMT node, so write the empty root here: the next
        // merklize reads the root at `version`, and the next open discovers
        // the snapshot by it.
        if expected_root_hash == *SPARSE_MERKLE_PLACEHOLDER_HASH {
            self.merkle_db.write_node_batch(&HashMap::from([(
                NodeKey::new_empty_path(version),
                Node::Null,
            )]))?;
            reseat_index(None)?;
        } else {
            ensure!(
                self.merkle_db
                    .latest_snapshot_version_at_or_before(version)?
                    == Some(version),
                "Fast sync restored no {} JMT snapshot at version {version}, but the target \
                 transaction info commits root {expected_root_hash:?}.",
                T::NAME,
            );
            let root_hash = self.merkle_db.get_root_hash(version)?;
            ensure!(
                root_hash == expected_root_hash,
                "Restored {} root {root_hash:?} at version {version} does not match the proved \
                 root {expected_root_hash:?}.",
                T::NAME,
            );
            reseat_index(Some(self.snapshot_rows(version)?))?;
        }
        self.kv_db.write_progress(version)?;

        if let (Some(store), Some(persisted), Some(pruner)) = (
            self.state_store.as_ref(),
            self.persisted.as_ref(),
            self.pruner.as_ref(),
        ) {
            store.reset_at_snapshot(
                Arc::clone(&self.merkle_db),
                Arc::clone(ledger_db),
                native_state_at_version::<T>(version, expected_root_hash),
                Arc::clone(pruner),
                persisted.clone(),
            );
        }
        Ok(())
    }

    /// Persists the chunk's value rows and stale index. Returns the chunk's
    /// last version, or `None` for an empty chunk.
    pub(crate) fn commit_kv(&self, chunk: &ChunkToCommit) -> Result<Option<Version>> {
        if chunk.transaction_outputs.is_empty() {
            return Ok(None);
        }
        let committer = NativeStateCommitter::new(Arc::clone(&self.kv_db));
        let chunk_first = chunk.first_version;
        let chunk_last_inclusive = chunk_first + chunk.transaction_outputs.len() as Version - 1;

        let mut sharded_kv_batches = new_sharded_kv_batches();
        let mut in_chunk_prior = InChunkPriorVersions::new();
        for (i, output) in chunk.transaction_outputs.iter().enumerate() {
            committer.apply(
                chunk_first + i as Version,
                T::write_set_ops(output.write_set()),
                &mut sharded_kv_batches,
                &mut in_chunk_prior,
            )?;
        }
        self.kv_db
            .commit(chunk_last_inclusive, None, sharded_kv_batches)?;
        Ok(Some(chunk_last_inclusive))
    }

    /// Advances the pipeline (merklize + persist) by the chunk. `summary` is
    /// the one execution computed when the store's root is consensus
    /// critical; without it the summary is computed here so the tree still
    /// tracks forward.
    pub(crate) fn advance_pipeline(
        &self,
        chunk: &ChunkToCommit,
        summary: Option<&NativeLedgerStateWithSummary>,
        sync_commit: bool,
    ) -> Result<()> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(());
        };
        let new_state = match summary {
            Some(summary) => summary.clone(),
            None => self.summary_at_commit(store, chunk)?,
        };
        store.buffered_state_locked().update(
            new_state,
            (),
            chunk.transaction_outputs.len(),
            sync_commit || chunk.is_reconfig,
        )
    }

    /// Extends the in-memory tip to the chunk's last version with its writes,
    /// freezing on the persisted base, and checkpointing where the chunk does.
    fn summary_at_commit(
        &self,
        store: &NativeStateStore,
        chunk: &ChunkToCommit,
    ) -> Result<NativeLedgerStateWithSummary> {
        let persisted_base = self
            .persisted
            .as_ref()
            .expect("persisted present when state_store is")
            .get();

        let (mut latest, mut last_checkpoint) = {
            let state = store.current_state();
            let current = state.lock();
            (current.latest().clone(), current.last_checkpoint().clone())
        };

        let chunk_first = chunk.first_version;
        let chunk_last_inclusive = chunk_first + chunk.transaction_outputs.len() as Version - 1;
        let checkpoint_within_chunk = chunk
            .state
            .last_checkpoint()
            .version()
            .filter(|v| (chunk_first..=chunk_last_inclusive).contains(v));

        let mut pending: HashMap<HashValue, PositionSlot> = HashMap::new();
        let extend_on_base = |latest: &NativeStateWithSummary,
                              version: Version,
                              updates: Vec<(HashValue, PositionSlot)>|
         -> Result<NativeStateWithSummary> {
            let proof_reader = NativeProofReader {
                merkle_db: Arc::clone(&self.merkle_db),
                version: persisted_base.version(),
            };
            latest.extend(version, updates, &persisted_base, &proof_reader)
        };

        for (i, output) in chunk.transaction_outputs.iter().enumerate() {
            let version = chunk_first + i as Version;
            for (key, op) in T::write_set_ops(output.write_set()) {
                pending.insert(key.hash(), leaf_update(key, op.as_state_value_opt()));
            }
            if Some(version) == checkpoint_within_chunk {
                let updates: Vec<_> = std::mem::take(&mut pending).into_iter().collect();
                latest = extend_on_base(&latest, version, updates)?;
                last_checkpoint = latest.clone();
            }
        }
        // Extended even without writes, so the tree follows the chain and
        // persists snapshots at its checkpoints. A tree stuck below the chain
        // never snapshots, and a reopen would replay from version 0.
        if latest.version() != Some(chunk_last_inclusive) {
            let updates: Vec<_> = pending.into_iter().collect();
            latest = extend_on_base(&latest, chunk_last_inclusive, updates)?;
        }

        Ok(NativeLedgerStateWithSummary::from_latest_and_last_checkpoint(latest, last_checkpoint))
    }
}

fn leaf_update(key: &StateKey, value: Option<&StateValue>) -> PositionSlot {
    PositionSlot {
        state_key: key.clone(),
        value_hash: value.map(CryptoHash::hash),
        value: None,
    }
}

/// The latest-wins JMT leaf update per key across `write_sets`.
fn latest_leaf_updates<T: NativeStoreSpec>(
    write_sets: &[WriteSet],
) -> HashMap<HashValue, PositionSlot> {
    write_sets
        .iter()
        .flat_map(T::write_set_ops)
        .map(|(key, op)| (key.hash(), leaf_update(key, op.as_state_value_opt())))
        .collect()
}
