// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::state_store::state_delta::StateDelta;
use derive_more::{Deref, DerefMut};

#[derive(Clone, Debug, Deref, DerefMut)]
pub(crate) struct CurrentState {
    #[deref]
    #[deref_mut]
    from_latest_checkpoint_to_current: StateDelta,
}

impl CurrentState {
    pub fn new_dummy() -> Self {
        Self {
            from_latest_checkpoint_to_current: StateDelta::new_empty(),
        }
    }

    pub fn set(&mut self, from_latest_checkpoint_to_current: StateDelta) {
        self.from_latest_checkpoint_to_current = from_latest_checkpoint_to_current;
    }

    pub fn get(&self) -> &StateDelta {
        &self.from_latest_checkpoint_to_current
    }
}
