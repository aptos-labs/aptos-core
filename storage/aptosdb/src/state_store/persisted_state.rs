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
use aptos_types::transaction::Version;
use std::sync::Arc;

struct FenceState {
    /// `None` = no fence (state sync mode, all commits pass through).
    /// `Some(v)` = hot state commits for versions > v are buffered.
    fence_version: Option<Version>,
    /// At most one pending hot state commit awaiting fence advancement.
    pending: Option<State>,
}

#[derive(Clone)]
pub struct PersistedState {
    hot_state: Arc<HotState>,
    summary: Arc<Mutex<StateSummary>>,
    fence: Arc<Mutex<FenceState>>,
}

impl PersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new_empty(config: HotStateConfig) -> Self {
        let state = State::new_empty(config);
        let hot_state = Arc::new(HotState::new(state, config));
        let summary = Arc::new(Mutex::new(StateSummary::new_empty(config)));
        let fence = Arc::new(Mutex::new(FenceState {
            fence_version: None,
            pending: None,
        }));
        Self {
            hot_state,
            summary,
            fence,
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

    pub fn set(&self, persisted: StateWithSummary) {
        let (state, summary) = persisted.into_inner();

        // n.b. Summary must be updated before committing the hot state, otherwise in the execution
        // pipeline we risk having a state generated based on a persisted version (v2) that's newer
        // than that of the summary (v1). That causes issue down the line where we commit the diffs
        // between a later snapshot (v3) and a persisted snapshot (v1) to the JMT, at which point
        // we will not be able to calculate the difference (v1 - v3) because the state links only
        // to as far as v2 (code will panic)
        *self.summary.lock() = summary;

        // Gate the hot state commit behind the fence. If the state version is beyond the fence,
        // buffer it instead of sending to the Committer thread. This prevents a race where the
        // Committer modifies the DashMap along one fork while a speculative block on a different
        // fork reads from it.
        let state_to_commit = {
            let mut fence = self.fence.lock();
            match fence.fence_version {
                Some(fence_ver) if state.version().is_some_and(|v| v > fence_ver) => {
                    fence.pending = Some(state);
                    None
                },
                _ => Some(state),
            }
        };

        if let Some(state) = state_to_commit {
            self.hot_state.enqueue_commit(state);
        }
    }

    /// Advance the hot state fence. If there is a pending state at or below the new fence,
    /// flush it to the Committer.
    pub fn advance_hot_state_fence(&self, version: Version) {
        let state_to_commit = {
            let mut fence = self.fence.lock();
            fence.fence_version = Some(version);
            match fence.pending.take() {
                Some(pending) if pending.version().is_some_and(|v| v <= version) => Some(pending),
                other => {
                    fence.pending = other;
                    None
                },
            }
        };

        if let Some(state) = state_to_commit {
            self.hot_state.enqueue_commit(state);
        }
    }

    // n.b. Can only be used when no on the fly commit is in the queue.
    pub fn hack_reset(&self, state_with_summary: StateWithSummary) {
        let (state, summary) = state_with_summary.into_inner();
        *self.summary.lock() = summary;
        self.fence.lock().pending = None;
        self.hot_state.set_commited(state);
    }
}
