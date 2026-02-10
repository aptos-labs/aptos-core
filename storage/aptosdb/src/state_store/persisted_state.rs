// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{metrics::OTHER_TIMERS_SECONDS, state_store::hot_state::HotState};
use aptos_config::config::HotStateConfig;
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::SUBTREE_DROPPER;
use aptos_storage_interface::state_store::{
    state::State, state_summary::StateSummary, state_view::hot_state_view::HotStateView,
    state_with_summary::StateWithSummary,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct PersistedState {
    hot_state: Arc<HotState>,
    summary: Arc<Mutex<StateSummary>>,
    /// The furthest state that the block executor has declared safe to expose through the hot
    /// state. Updated only after all in-flight speculative executions up to that version complete.
    hot_state_progress: Arc<Mutex<State>>,
}

impl PersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new_empty(config: HotStateConfig) -> Self {
        let state = State::new_empty(config);
        let hot_state = Arc::new(HotState::new(state.clone(), config));
        let summary = Arc::new(Mutex::new(StateSummary::new_empty(config)));
        let hot_state_progress = Arc::new(Mutex::new(state));
        Self {
            hot_state,
            summary,
            hot_state_progress,
        }
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

    /// Advance the hot state progress. The block executor calls this once all in-flight
    /// speculative executions up to this version have finished, so it is safe to expose this
    /// state through the hot state.
    pub fn set_hot_state_progress(&self, state: State) {
        *self.hot_state_progress.lock() = state;
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

        // Gate: only advance the hot state up to what the execution layer has marked safe.
        // If `state` is ahead of `allowed_progress`, a fork branch may still be executing
        // against the old committed data, so we must not expose the newer state yet.
        let allowed_progress = self.hot_state_progress.lock().clone();
        let to_commit = if allowed_progress.next_version() < state.next_version() {
            assert!(
                state.is_descendant_of(&allowed_progress),
                "Persisted state (version {:?}) is not a descendant of allowed_progress (version {:?}).",
                state.version(),
                allowed_progress.version(),
            );
            allowed_progress
        } else {
            state
        };
        self.hot_state.enqueue_commit(to_commit);
    }

    // n.b. Can only be used when no on the fly commit is in the queue.
    pub fn hack_reset(&self, state_with_summary: StateWithSummary) {
        let (state, summary) = state_with_summary.into_inner();
        *self.summary.lock() = summary;
        *self.hot_state_progress.lock() = state.clone();
        self.hot_state.set_commited(state);
    }
}
