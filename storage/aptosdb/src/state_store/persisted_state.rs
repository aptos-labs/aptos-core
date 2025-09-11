// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::OTHER_TIMERS_SECONDS, state_store::hot_state::HotState};
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::SUBTREE_DROPPER;
use aptos_storage_interface::state_store::{
    state::State, state_summary::StateSummary, state_view::hot_state_view::HotStateView,
    state_with_summary::StateWithSummary,
};
use aptos_types::state_store::hot_state::{
    HOT_STATE_MAX_BYTES_PER_SHARD, HOT_STATE_MAX_ITEMS_PER_SHARD, HOT_STATE_MAX_SINGLE_VALUE_BYTES,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct PersistedState {
    hot_state: Arc<HotState>,
    summary: Arc<Mutex<StateSummary>>,
}

impl PersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new_empty() -> Self {
        Self::new_empty_with_config(
            HOT_STATE_MAX_ITEMS_PER_SHARD.get(),
            HOT_STATE_MAX_BYTES_PER_SHARD,
            HOT_STATE_MAX_SINGLE_VALUE_BYTES,
        )
    }

    pub fn new_empty_with_config(
        hot_state_max_items_per_shard: usize,
        hot_state_max_bytes_per_shard: usize,
        hot_state_max_single_value_bytes: usize,
    ) -> Self {
        let state = State::new_empty();
        let hot_state = Arc::new(HotState::new(
            state,
            hot_state_max_items_per_shard,
            hot_state_max_bytes_per_shard,
            hot_state_max_single_value_bytes,
        ));
        let summary = Arc::new(Mutex::new(StateSummary::new_empty()));
        Self { hot_state, summary }
    }

    pub fn get_state_summary(&self) -> StateSummary {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["get_persisted_state_summary"]);

        // The back pressure is on the getting side (which is the execution side) so that it's less
        // likely for a lot of blocks locking the same old base SMT.
        SUBTREE_DROPPER.wait_for_backlog_drop(Self::MAX_PENDING_DROPS);

        self.summary.lock().clone()
    }

    #[cfg(test)]
    pub fn get_hot_state(&self) -> Arc<HotState> {
        Arc::clone(&self.hot_state)
    }

    pub fn get_state(&self) -> (Arc<dyn HotStateView>, State) {
        self.hot_state.get_committed()
    }

    pub fn set(&self, persisted: StateWithSummary) {
        let (state, summary) = persisted.into_inner();

        // n.b. Summary must be updated before committing the hot state, otherwise in the execution
        // pipeline we risk having a state generated based on a persisted version (v2) that's newer
        // than that of the summary (v1). That causes issue down the line where we commit the diffs
        // between a later snapshot (v3) and a persisted snapshot (v1) to the JMT, at which point
        // we will not be able to calculate the difference (v1 - v3) because the state links only
        // to as far as v2 (code will panic)
        *self.summary.lock() = summary;

        self.hot_state.enqueue_commit(state);
    }

    // n.b. Can only be used when no on the fly commit is in the queue.
    pub fn hack_reset(&self, state_with_summary: StateWithSummary) {
        let (state, summary) = state_with_summary.into_inner();
        *self.summary.lock() = summary;
        self.hot_state.set_commited(state);
    }
}
