// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_delta::StateDelta;
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{state_store::state_value::StateValue, transaction::Version};

/// The data structure through which the entire state at a given
/// version can be summarized to a concise digest (the root hash).
pub struct StateSummary {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    pub global_state_summary: SparseMerkleTree<StateValue>,
}

impl StateSummary {
    pub fn new(next_version: Version, global_state_summary: SparseMerkleTree<StateValue>) -> Self {
        Self {
            next_version,
            global_state_summary,
        }
    }

    pub fn update(&self, _persisted: &StateSummary, _state_delta: &StateDelta) -> Self {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn root_hash(&self) -> HashValue {
        self.global_state_summary.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }
}
