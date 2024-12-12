// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS_SECONDS;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::SUBTREE_DROPPER;
use aptos_storage_interface::state_store::state_summary::StateWithSummary;
use std::ops::Deref;

pub struct PersistedState {
    persisted: StateWithSummary,
}

impl PersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new_dummy() -> Self {
        Self {
            persisted: StateWithSummary::new_empty(),
        }
    }

    pub fn get(&self) -> &StateWithSummary {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["get_persisted_state"]);

        // The back pressure is on the getting side (which is the execution side) so that it's less
        // likely for a lot of blocks locking the same old base SMT.
        SUBTREE_DROPPER.wait_for_backlog_drop(Self::MAX_PENDING_DROPS);

        &self.persisted
    }

    pub fn set(&mut self, persisted: StateWithSummary) {
        self.persisted = persisted;
    }
}

impl Deref for PersistedState {
    type Target = StateWithSummary;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
