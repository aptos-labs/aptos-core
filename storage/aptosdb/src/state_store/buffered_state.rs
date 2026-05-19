// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Main-state buffered state. Thin façade over the generic
//! [`crate::common::BufferedState`] — the only state-specific piece
//! still here is [`HotStateAccumulator`], which provides the
//! pre/post-checkpoint hot-state accumulation that gets folded into
//! each [`SnapshotToCommit`].

use crate::{
    common::{spawn_commit_pipeline, BufferedStateCore, BufferedStateExtras},
    metrics::{LATEST_CHECKPOINT_VERSION, OTHER_TIMERS_SECONDS},
    state_store::{
        persisted_state::PersistedState,
        state_merkle_batch_committer::StateMerkleBatchCommitter,
        state_snapshot_committer::{
            merklize_main_state, SnapshotToCommit, STATE_BATCH_CHANNEL_SIZE,
        },
        StateDb,
    },
};
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    empty_hot_state_updates,
    state_with_summary::{LedgerStateWithSummary, StateWithSummary},
    HotStateShardUpdates, HotStateUpdates,
};
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};
use itertools::Itertools;
use std::sync::Arc;

pub(crate) const ASYNC_COMMIT_CHANNEL_BUFFER_SIZE: u64 = 1;
pub(crate) const TARGET_SNAPSHOT_INTERVAL_IN_VERSION: u64 = 100_000;

/// Buffered state for main state. Composes the shared generic with
/// [`HotStateAccumulator`] for the per-pipeline hot-state extras.
pub type BufferedState = crate::common::BufferedState<
    LedgerStateWithSummary,
    StateWithSummary,
    SnapshotToCommit,
    HotStateAccumulator,
>;

/// Hot-state pre/post-checkpoint accumulator. Implements
/// [`BufferedStateExtras`] for main state so that the snapshot
/// committer receives the right `hot_state_updates` bundled into each
/// [`SnapshotToCommit`].
///
/// Two buckets:
/// - `pending`: hot-state updates covering `(last_snapshot, current.last_checkpoint()]`.
///   Drained into the payload on every snapshot flush.
/// - `post_checkpoint`: hot-state updates covering
///   `(current.last_checkpoint(), current.latest()]`. Folded into
///   `pending` once a later chunk advances the checkpoint past them.
pub struct HotStateAccumulator {
    pending: [HotStateShardUpdates; NUM_STATE_SHARDS],
    post_checkpoint: [HotStateShardUpdates; NUM_STATE_SHARDS],
}

impl HotStateAccumulator {
    pub fn new() -> Self {
        Self {
            pending: empty_hot_state_updates(),
            post_checkpoint: empty_hot_state_updates(),
        }
    }

    /// Merges `incoming` into `target` per-shard.
    fn merge_hot_state_updates(
        target: &mut [HotStateShardUpdates; NUM_STATE_SHARDS],
        incoming: [HotStateShardUpdates; NUM_STATE_SHARDS],
    ) {
        for (t, i) in target.iter_mut().zip_eq(incoming.into_iter()) {
            t.merge(i);
        }
    }
}

impl Default for HotStateAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferedStateExtras<SnapshotToCommit, StateWithSummary> for HotStateAccumulator {
    type ChunkInput = HotStateUpdates;

    /// Phase order:
    /// 1. If `checkpoint_advanced`, fold the previous chunk's
    ///    `post_checkpoint` accumulation into `pending` (those updates
    ///    now precede the new checkpoint).
    /// 2. Merge this chunk's `for_last_checkpoint` portion into
    ///    `pending`. After this `build_payload` (if invoked) will see
    ///    all pre-checkpoint updates inclusive of this chunk's
    ///    contribution.
    /// 3. Merge this chunk's `for_latest` portion into
    ///    `post_checkpoint` so it is preserved across the upcoming
    ///    `build_payload` and folded into the next pending bucket when
    ///    a future chunk advances the checkpoint.
    fn absorb_chunk(&mut self, input: HotStateUpdates, checkpoint_advanced: bool) {
        if checkpoint_advanced {
            let prev_post = std::mem::replace(&mut self.post_checkpoint, empty_hot_state_updates());
            Self::merge_hot_state_updates(&mut self.pending, prev_post);
        }
        if let Some(shards) = input.for_last_checkpoint {
            Self::merge_hot_state_updates(&mut self.pending, shards);
        }
        if let Some(shards) = input.for_latest {
            Self::merge_hot_state_updates(&mut self.post_checkpoint, shards);
        }
    }

    fn build_payload(&mut self, snapshot: StateWithSummary) -> SnapshotToCommit {
        let hot_state_updates = std::mem::replace(&mut self.pending, empty_hot_state_updates());
        SnapshotToCommit {
            snapshot,
            hot_state_updates,
        }
    }
}

impl BufferedState {
    /// Construct a `BufferedState` at the snapshot version. State-specific
    /// dependencies (state_db, persisted state) thread through here; the
    /// generic [`crate::common::BufferedState`] doesn't need to know about
    /// them.
    pub(crate) fn new_at_snapshot(
        state_db: &Arc<StateDb>,
        last_snapshot: StateWithSummary,
        target_items: usize,
        out_current_state: Arc<Mutex<LedgerStateWithSummary>>,
        out_persisted_state: PersistedState,
    ) -> Self {
        new_at_snapshot_impl(
            state_db,
            last_snapshot,
            target_items,
            out_current_state,
            out_persisted_state,
        )
    }
}

fn new_at_snapshot_impl(
    state_db: &Arc<StateDb>,
    last_snapshot: StateWithSummary,
    target_items: usize,
    out_current_state: Arc<Mutex<LedgerStateWithSummary>>,
    out_persisted_state: PersistedState,
) -> BufferedState {
    let arc_state_db = Arc::clone(state_db);
    *out_current_state.lock() = LedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone());
    out_persisted_state.hack_reset(last_snapshot.clone());

    let merklize_state_db = Arc::clone(&arc_state_db);
    let persisted_state_clone = out_persisted_state.clone();
    let commit_thread = spawn_commit_pipeline(
        "state-committer",
        ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize,
        "state_batch_committer",
        STATE_BATCH_CHANNEL_SIZE,
        last_snapshot.clone(),
        move |batch_receiver| {
            StateMerkleBatchCommitter::new(arc_state_db, batch_receiver, persisted_state_clone)
                .run();
        },
        move |last_snapshot, input| merklize_main_state(&merklize_state_db, last_snapshot, input),
    );
    report_last_checkpoint_version(last_snapshot.version());
    BufferedState::new(
        BufferedStateCore::new(
            out_current_state,
            last_snapshot,
            commit_thread,
            target_items,
            TARGET_SNAPSHOT_INTERVAL_IN_VERSION,
        ),
        HotStateAccumulator::new(),
    )
}

/// Wrapper around `BufferedState::update` that also reports the
/// last-checkpoint metric and adapts the timer label. Callers
/// (state-side commit path) should invoke this rather than
/// `BufferedState::update` directly so the metric stays in lockstep.
pub(crate) fn update(
    state: &mut BufferedState,
    new_state: LedgerStateWithSummary,
    hot_state_updates: HotStateUpdates,
    estimated_new_items: usize,
    sync_commit: bool,
) -> aptos_storage_interface::Result<()> {
    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___update"]);
    let version = new_state.last_checkpoint().version();
    state.update(
        new_state,
        hot_state_updates,
        estimated_new_items,
        sync_commit,
    )?;
    report_last_checkpoint_version(version);
    Ok(())
}

fn report_last_checkpoint_version(version: Option<Version>) {
    LATEST_CHECKPOINT_VERSION.set(version.map_or(-1, |v| v as i64));
}
