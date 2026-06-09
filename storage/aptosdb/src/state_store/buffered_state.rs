// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
use aptos_types::state_store::NUM_STATE_SHARDS;
use itertools::Itertools;
use std::sync::Arc;

pub(crate) const ASYNC_COMMIT_CHANNEL_BUFFER_SIZE: u64 = 1;
pub(crate) const TARGET_SNAPSHOT_INTERVAL_IN_VERSION: u64 = 100_000;

pub type BufferedState = crate::common::BufferedState<
    LedgerStateWithSummary,
    StateWithSummary,
    SnapshotToCommit,
    HotStateAccumulator,
>;

/// `pending` covers `(last_snapshot, current.last_checkpoint()]` and
/// drains into the snapshot payload on every flush. `post_checkpoint`
/// covers `(last_checkpoint, latest()]` and folds into `pending` once
/// a later chunk advances the checkpoint past it.
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

    fn absorb_chunk(&mut self, input: HotStateUpdates, checkpoint_advanced: bool) {
        // Order: (1) fold prior post_checkpoint into pending if the
        // checkpoint advanced past it, (2) merge this chunk's
        // pre-checkpoint share into pending, (3) merge this chunk's
        // post-checkpoint share into post_checkpoint.
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
    pub(crate) fn new_at_snapshot(
        state_db: &Arc<StateDb>,
        last_snapshot: StateWithSummary,
        target_items: usize,
        out_current_state: Arc<Mutex<LedgerStateWithSummary>>,
        out_persisted_state: PersistedState,
    ) -> Self {
        let arc_state_db = Arc::clone(state_db);
        *out_current_state.lock() =
            LedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone());
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
            move |last_snapshot, input| {
                merklize_main_state(&merklize_state_db, last_snapshot, input)
            },
        );
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

    /// Calls `update` and reports `LATEST_CHECKPOINT_VERSION` so the metric
    /// stays in lockstep with the snapshot's checkpoint version.
    pub(crate) fn update_and_report(
        &mut self,
        new_state: LedgerStateWithSummary,
        hot_state_updates: HotStateUpdates,
        estimated_new_items: usize,
        sync_commit: bool,
    ) -> aptos_storage_interface::Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___update"]);
        let version = new_state.last_checkpoint().version();
        self.update(
            new_state,
            hot_state_updates,
            estimated_new_items,
            sync_commit,
        )?;
        LATEST_CHECKPOINT_VERSION.set(version.map_or(-1, |v| v as i64));
        Ok(())
    }
}
