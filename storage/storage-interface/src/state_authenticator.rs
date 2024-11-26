// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_delta::StateDelta;
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{state_store::state_value::StateValue, transaction::Version};

/// note: only a single field for now, more to be introduced later.
#[derive(Clone, Debug)]
pub struct StateAuthenticator {
    next_version: Version,
    pub global_state: SparseMerkleTree<StateValue>,
}

impl StateAuthenticator {
    pub fn new(next_version: Version, global_state: SparseMerkleTree<StateValue>) -> Self {
        Self {
            next_version,
            global_state,
        }
    }

    pub fn update(&self, _persisted_auth: &StateAuthenticator, _state_delta: &StateDelta) -> Self {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn root_hash(&self) -> HashValue {
        self.global_state.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }
}
