// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::{
        spawn_commit_pipeline, BufferedStateCore, BufferedStateExtras, CheckpointSnapshot,
        LedgerStateView,
    },
    ledger_db::LedgerDb,
    position_merkle_batch_committer::PositionMerkleBatchCommitter,
    position_merkle_db::PositionMerkleDb,
    position_snapshot_committer::{
        merklize_position, PositionSnapshotToCommit, POSITION_BATCH_CHANNEL_SIZE,
    },
    state_store::buffered_state::{
        ASYNC_COMMIT_CHANNEL_BUFFER_SIZE, TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
    },
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::JellyfishMerkleTree;
use aptos_scratchpad::{ProofRead, SparseMerkleTree};
use aptos_storage_interface::state_store::{
    sharded_jmt_state::{LeafSlot, ShardedJmtState},
    state_summary::StateSummary,
    state_with_summary::{LedgerWithSummary, StateAndSummary},
};
use aptos_types::{proof::SparseMerkleProofExt, transaction::Version};
use std::sync::Arc;

pub type PositionSlot = LeafSlot<()>;

pub type PositionStateSummary = StateSummary;

pub type PositionStateWithSummary = StateAndSummary<ShardedJmtState<PositionSlot>>;

pub fn new_empty_position_state() -> PositionStateWithSummary {
    PositionStateWithSummary::new_empty("position")
}

/// Restart-from-disk constructor: seeds the in-memory pipeline with
/// the JMT root persisted at `version`.
pub fn position_state_at_version(
    version: Version,
    root_hash: HashValue,
) -> PositionStateWithSummary {
    PositionStateWithSummary::new(
        ShardedJmtState::new_at_version(Some(version), "position"),
        StateSummary::new_global_only(version, SparseMerkleTree::new(root_hash)),
    )
}

pub type PositionLedgerStateWithSummary = LedgerWithSummary<PositionStateWithSummary>;

impl CheckpointSnapshot for PositionStateWithSummary {
    fn next_version(&self) -> Version {
        self.next_version()
    }
}

impl LedgerStateView for PositionLedgerStateWithSummary {
    type Snapshot = PositionStateWithSummary;

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

pub struct PositionProofReader {
    pub merkle_db: Arc<PositionMerkleDb>,
    /// `None` before any snapshot has been persisted; in that case
    /// the SMT extend path never reaches the proof reader.
    pub version: Option<Version>,
}

impl ProofRead for PositionProofReader {
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt> {
        let version = self.version?;
        let tree = JellyfishMerkleTree::new(self.merkle_db.as_ref());
        let (_value, proof) = tree
            .get_with_proof_ext(key, version, root_depth)
            .expect("Failed to get position state proof by version.");
        Some(proof)
    }
}

pub(crate) const POSITION_TARGET_ITEMS: usize = 200_000;

pub(crate) type PositionBufferedState = crate::common::BufferedState<
    PositionLedgerStateWithSummary,
    PositionStateWithSummary,
    PositionSnapshotToCommit,
    PositionExtras,
>;

pub struct PositionExtras;

impl BufferedStateExtras<PositionSnapshotToCommit, PositionStateWithSummary> for PositionExtras {
    type ChunkInput = ();

    fn absorb_chunk(&mut self, (): (), _checkpoint_advanced: bool) {}

    fn build_payload(&mut self, snapshot: PositionStateWithSummary) -> PositionSnapshotToCommit {
        PositionSnapshotToCommit { snapshot }
    }
}

impl PositionBufferedState {
    pub fn new_at_snapshot(
        merkle_db: Arc<PositionMerkleDb>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: PositionStateWithSummary,
        target_items: usize,
        out_current_state: Arc<Mutex<PositionLedgerStateWithSummary>>,
    ) -> Self {
        *out_current_state.lock() =
            PositionLedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone());

        let snapshot_merkle_db = Arc::clone(&merkle_db);
        let snapshot_ledger_db = Arc::clone(&ledger_db);
        let batch_merkle_db = Arc::clone(&merkle_db);
        let commit_thread = spawn_commit_pipeline(
            "position_snapshot_committer",
            ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize,
            "position_merkle_batch_committer",
            POSITION_BATCH_CHANNEL_SIZE,
            last_snapshot.clone(),
            move |batch_receiver| {
                PositionMerkleBatchCommitter::new(batch_merkle_db, batch_receiver).run();
            },
            move |last_snapshot, input| {
                merklize_position(
                    &snapshot_merkle_db,
                    &snapshot_ledger_db,
                    last_snapshot,
                    input,
                )
                .expect("Failed to compute position JMT commit batch.")
            },
        );

        PositionBufferedState::new(
            BufferedStateCore::new(
                out_current_state,
                last_snapshot,
                commit_thread,
                target_items,
                TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
            ),
            PositionExtras,
        )
    }
}
