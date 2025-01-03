// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS_SECONDS;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::{SparseMerkleTree, SUBTREE_DROPPER};
use aptos_types::state_store::state_value::StateValue;
use std::ops::Deref;

pub struct PersistedState {
    smt: SparseMerkleTree<StateValue>,
}

impl PersistedState {
    const MAX_PENDING_DROPS: usize = 8;

    pub fn new_dummy() -> Self {
        Self {
            smt: SparseMerkleTree::new_empty(),
        }
    }

    pub fn get(&self) -> &SparseMerkleTree<StateValue> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["get_persisted_state"]);

        // The back pressure is on the getting side (which is the execution side) so that it's less
        // likely for a lot of blocks locking the same old base SMT.
        SUBTREE_DROPPER.wait_for_backlog_drop(Self::MAX_PENDING_DROPS);

        &self.smt
    }

    pub fn set(&mut self, smt: SparseMerkleTree<StateValue>) {
        self.smt = smt
    }
}

impl Deref for PersistedState {
    type Target = SparseMerkleTree<StateValue>;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
