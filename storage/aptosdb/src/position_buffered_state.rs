// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! In-memory state with summary + async commit driver for the native-
//! position JMT. Mirrors [`crate::state_store::buffered_state::BufferedState`]
//! point-for-point, **minus the hot/cold split** (position has no
//! hot state — recently-accessed positions live in `NativeStateStore`
//! independently).
//!
//! Update flow — mirrors main state: callers build a new
//! [`PositionLedgerStateWithSummary`] upstream via
//! [`PositionStateWithSummary::extend`], then call
//! [`PositionBufferedState::update`]. The snapshot committer
//! extracts the delta to drain via [`PositionStateWithSummary::make_delta`]
//! against its `last_snapshot` — same shape main state uses with
//! `State::make_delta`.

#![forbid(unsafe_code)]

use crate::{
    common::{spawn_commit_pipeline, BufferedStateCore, BufferedStateExtras},
    ledger_db::LedgerDb,
    position_merkle_batch_committer::PositionMerkleBatchCommitter,
    position_merkle_db::PositionMerkleDb,
    position_snapshot_committer::{
        merklize_position, PositionSnapshotToCommit, POSITION_BATCH_CHANNEL_SIZE,
    },
    position_state::{PositionSlot, POSITION_STATE_FAMILY},
    state_store::buffered_state::{
        ASYNC_COMMIT_CHANNEL_BUFFER_SIZE, TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
    },
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::JellyfishMerkleTree;
use aptos_scratchpad::ProofRead;
use aptos_storage_interface::state_store::{
    jmt_pipeline::ShardedJmtState,
    state_summary::StateSummary,
    state_with_summary::{LedgerWithSummary, StateAndSummary},
};
use aptos_types::{proof::SparseMerkleProofExt, transaction::Version};
use std::sync::Arc;

/// Position's per-version summary — type alias of the shared
/// [`StateSummary`] with `hot_state_summary = None` (position has no
/// hot companion). Construction (`new_empty_global_only` /
/// `new_global_only`) and update primitives (`SmtSummary` on
/// `global_state_summary`) live on the generic.
pub type PositionStateSummary = StateSummary;

/// Position's `{ state, summary }` pair at one version — type alias
/// of the shared `StateAndSummary` over a `ShardedJmtState<PositionSlot>`.
/// All construction / extend / delta operations live on the inherent
/// impl in `storage-interface`. Construct fresh empty instances via
/// [`new_empty_position_state`].
pub type PositionStateWithSummary = StateAndSummary<ShardedJmtState<PositionSlot>>;

/// Pre-genesis empty position state — wraps the generic's
/// `new_empty` with the right MapLayer family tag.
pub fn new_empty_position_state() -> PositionStateWithSummary {
    PositionStateWithSummary::new_empty(POSITION_STATE_FAMILY)
}

/// Latest + last-checkpoint pair. Type alias of the generic
/// [`LedgerWithSummary`] — for now position treats every committed
/// version as a checkpoint (`latest == last_checkpoint`). Construct
/// via `new_at_checkpoint` / `from_latest_and_last_checkpoint` from
/// the generic; access pair components via `latest()` /
/// `last_checkpoint()` accessors.
pub type PositionLedgerStateWithSummary = LedgerWithSummary<PositionStateWithSummary>;

/// `ProofRead` impl backed by `position_merkle_db`. The scratchpad
/// SMT consults this for keys whose proof path isn't materialized
/// in-memory yet.
pub struct PositionProofReader {
    pub merkle_db: Arc<PositionMerkleDb>,
    pub version: Version,
}

impl ProofRead for PositionProofReader {
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt> {
        let tree = JellyfishMerkleTree::new(self.merkle_db.as_ref());
        match tree.get_with_proof_ext(key, self.version, root_depth) {
            Ok((_value, proof)) => Some(proof),
            Err(_) => None,
        }
    }
}

/// Cap on accumulated items before forcing a flush even if
/// `buffered_versions` hasn't crossed `TARGET_SNAPSHOT_INTERVAL`.
/// Mirrors `BufferedState`'s `target_items` budget.
pub(crate) const POSITION_TARGET_ITEMS: usize = 200_000;

/// Buffered native-position state. Same generic type as
/// [`crate::state_store::buffered_state::BufferedState`]; the only
/// difference is the per-pipeline `extras` parameter — position uses
/// the no-op [`PositionExtras`] while main state plugs in
/// `HotStateAccumulator`.
///
/// `update(new_state, (), estimated_new_items, sync_commit)` is the
/// entry point; the `()` is the (unused) chunk-input slot.
pub(crate) type PositionBufferedState = crate::common::BufferedState<
    PositionLedgerStateWithSummary,
    PositionStateWithSummary,
    PositionSnapshotToCommit,
    PositionExtras,
>;

/// Zero-sized extras for the position pipeline — no hot-state, no
/// pre/post-checkpoint accumulator, no per-chunk side data. Just
/// wraps the snapshot in a `PositionSnapshotToCommit`.
pub struct PositionExtras;

impl BufferedStateExtras<PositionSnapshotToCommit, PositionStateWithSummary> for PositionExtras {
    type ChunkInput = ();

    fn absorb_chunk(&mut self, (): (), _checkpoint_advanced: bool) {}

    fn build_payload(&mut self, snapshot: PositionStateWithSummary) -> PositionSnapshotToCommit {
        PositionSnapshotToCommit { snapshot }
    }
}

impl PositionBufferedState {
    /// Construct a buffered state at `last_snapshot`. Pipeline-specific
    /// dependencies (merkle DB, ledger DB) thread through here; the
    /// generic [`crate::common::BufferedState`] doesn't know about them.
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
