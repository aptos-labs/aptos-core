// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::state_store::{state::LedgerState, state_delta::StateDelta};
use derive_more::{Deref, DerefMut};

#[derive(Clone, Debug, Deref, DerefMut)]
pub(crate) struct CurrentState {
    #[deref]
    #[deref_mut]
    latest_state: LedgerState,
}

impl CurrentState {
    pub fn new_dummy() -> Self {
        Self {
            latest_state: LedgerState::new_empty(),
        }
    }

    pub fn set(&mut self, latest_state: LedgerState) {
        self.latest_state = latest_state
    }

    pub fn get(&self) -> &LedgerState {
        &self.latest_state
    }
}
