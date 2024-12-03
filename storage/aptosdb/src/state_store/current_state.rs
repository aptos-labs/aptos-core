// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::state_store::{state::LedgerState, state_delta::StateDelta};
use derive_more::{Deref, DerefMut};
use aptos_storage_interface::state_store::state_summary::LedgerStateSummary;

#[derive(Clone, Debug, Deref, DerefMut)]
pub(crate) struct CurrentState {
    state: LedgerState,
    state_summary: LedgerStateSummary,
}

impl CurrentState {
    pub fn new_dummy() -> Self {
        Self {
            state: LedgerState::new_empty(),
            state_summary: LedgerStateSummary::new_empty(),
        }
    }

    pub fn set(&mut self, state: LedgerState, state_summary: LedgerStateSummary) {
        self.state = state;
        self.state_summary = state_summary;
    }
}
