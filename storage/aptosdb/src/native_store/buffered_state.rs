// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::{
        spawn_commit_pipeline, BufferedStateCore, BufferedStateExtras, CheckpointSnapshot,
        LedgerStateView,
    },
    ledger_db::LedgerDb,
    native_store::{
        merkle_batch_committer::NativeMerkleBatchCommitter,
        merkle_db::NativeMerkleDb,
        pruner::NativeStorePruner,
        snapshot_committer::{merklize_native, NativeSnapshotToCommit, NATIVE_BATCH_CHANNEL_SIZE},
        NativeStoreSpec,
    },
    state_store::buffered_state::{
        ASYNC_COMMIT_CHANNEL_BUFFER_SIZE, TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
    },
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_scratchpad::{ProofRead, SparseMerkleTree, SUBTREE_DROPPER};
pub use aptos_storage_interface::state_store::sharded_jmt_state::{
    PositionSlot, PositionStateWithSummary,
};
use aptos_storage_interface::state_store::{
    sharded_jmt_state::ShardedJmtState, state_summary::StateSummary,
    state_with_summary::LedgerWithSummary,
};
use aptos_types::{proof::SparseMerkleProofExt, transaction::Version};
use std::sync::Arc;

/// A native store's in-memory JMT + SMT state. Leaves carry only the key and
/// value hash, so every store shares the position slot shape.
pub type NativeStateWithSummary = PositionStateWithSummary;
pub type NativeLedgerStateWithSummary = LedgerWithSummary<NativeStateWithSummary>;
pub type PositionLedgerStateWithSummary = NativeLedgerStateWithSummary;

pub fn new_empty_native_state<T: NativeStoreSpec>() -> NativeStateWithSummary {
    NativeStateWithSummary::new_empty(T::NAME)
}

/// Restart-from-disk constructor: seeds the in-memory pipeline with the
/// JMT root persisted at `version`.
pub fn native_state_at_version<T: NativeStoreSpec>(
    version: Version,
    root_hash: HashValue,
) -> NativeStateWithSummary {
    NativeStateWithSummary::new(
        ShardedJmtState::new_at_version(Some(version), T::NAME),
        StateSummary::new_global_only(version, SparseMerkleTree::new(root_hash)),
    )
}

impl CheckpointSnapshot for NativeStateWithSummary {
    fn next_version(&self) -> Version {
        self.next_version()
    }
}

impl LedgerStateView for NativeLedgerStateWithSummary {
    type Snapshot = NativeStateWithSummary;

    fn next_version(&self) -> Version {
        self.latest().next_version()
    }

    fn last_checkpoint_snapshot(&self) -> Self::Snapshot {
        self.last_checkpoint().clone()
    }

    fn is_descendant_of(&self, other: &Self) -> bool {
        self.latest().is_descendant_of(other.latest())
            && self
                .last_checkpoint()
                .is_descendant_of(other.last_checkpoint())
    }
}

pub struct NativeProofReader<T: NativeStoreSpec> {
    pub merkle_db: Arc<NativeMerkleDb<T>>,
    /// `None` before any snapshot has been persisted; in that case
    /// the SMT extend path never reaches the proof reader.
    pub version: Option<Version>,
}

impl<T: NativeStoreSpec> ProofRead for NativeProofReader<T> {
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt> {
        let version = self.version?;
        let (_value, proof) = self
            .merkle_db
            .get_with_proof_ext(key, version, root_depth)
            .unwrap_or_else(|e| panic!("Failed to get {} state proof by version: {e}", T::NAME));
        Some(proof)
    }
}

/// Shared handle to the latest persisted snapshot. The merkle batch
/// committer advances it (`set`) once a snapshot's JMT nodes are on disk;
/// execution reads it (`get`) as the SMT freeze base and proof version for
/// the next chunk. Holds the full summary so the version and the SMT stay
/// consistent under one lock.
#[derive(Clone)]
pub(crate) struct NativePersistedState {
    inner: Arc<Mutex<NativeStateWithSummary>>,
}

impl NativePersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub(crate) fn new(seed: NativeStateWithSummary) -> Self {
        Self {
            inner: Arc::new(Mutex::new(seed)),
        }
    }

    pub(crate) fn get(&self) -> NativeStateWithSummary {
        // Backpressure on the reader (execution) side so we don't pile
        // up a long chain of old base summaries pending subtree drop.
        SUBTREE_DROPPER.wait_for_backlog_drop(Self::MAX_PENDING_DROPS);
        self.inner.lock().clone()
    }

    pub(crate) fn set(&self, snapshot: NativeStateWithSummary) {
        *self.inner.lock() = snapshot;
    }
}

pub(crate) const NATIVE_TARGET_ITEMS: usize = 200_000;

/// The native-store counterpart of `state_store::MAX_WRITE_SETS_AFTER_SNAPSHOT`.
/// Cap on the startup replay window so a node enabling trading-native from
/// before its first snapshot doesn't load arbitrarily many historical write
/// sets into one `Vec`.
pub(crate) const MAX_NATIVE_WRITE_SETS_AFTER_SNAPSHOT: u64 =
    TARGET_SNAPSHOT_INTERVAL_IN_VERSION * (ASYNC_COMMIT_CHANNEL_BUFFER_SIZE + 2 + 1) * 2;

pub(crate) type NativeBufferedState = crate::common::BufferedState<
    NativeLedgerStateWithSummary,
    NativeStateWithSummary,
    NativeSnapshotToCommit,
    NativeExtras,
>;

pub struct NativeExtras;

impl BufferedStateExtras<NativeSnapshotToCommit, NativeStateWithSummary> for NativeExtras {
    type ChunkInput = ();

    fn absorb_chunk(&mut self, (): (), _checkpoint_advanced: bool) {}

    fn build_payload(&mut self, snapshot: NativeStateWithSummary) -> NativeSnapshotToCommit {
        NativeSnapshotToCommit { snapshot }
    }
}

impl NativeBufferedState {
    pub fn new_at_snapshot<T: NativeStoreSpec>(
        merkle_db: Arc<NativeMerkleDb<T>>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: NativeStateWithSummary,
        target_items: usize,
        out_current_state: Arc<Mutex<NativeLedgerStateWithSummary>>,
        pruner: Arc<NativeStorePruner<T>>,
        persisted: NativePersistedState,
    ) -> Self {
        *out_current_state.lock() =
            NativeLedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone());

        let snapshot_merkle_db = Arc::clone(&merkle_db);
        let snapshot_ledger_db = Arc::clone(&ledger_db);
        let batch_merkle_db = Arc::clone(&merkle_db);
        let commit_thread = spawn_commit_pipeline(
            T::SNAPSHOT_THREAD_NAME,
            ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize,
            T::BATCH_THREAD_NAME,
            NATIVE_BATCH_CHANNEL_SIZE,
            last_snapshot.clone(),
            move |batch_receiver| {
                NativeMerkleBatchCommitter::new(batch_merkle_db, batch_receiver, pruner, persisted)
                    .run();
            },
            move |last_snapshot, input| {
                merklize_native(
                    &snapshot_merkle_db,
                    &snapshot_ledger_db,
                    last_snapshot,
                    input,
                )
                .unwrap_or_else(|e| panic!("Failed to compute {} JMT commit batch: {e}", T::NAME))
            },
        );

        NativeBufferedState::new(
            BufferedStateCore::new(
                out_current_state,
                last_snapshot,
                commit_thread,
                target_items,
                TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
            ),
            NativeExtras,
        )
    }
}
