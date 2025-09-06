// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    state_store::{hot_state::LRUEntry, state_slot::StateSlot},
    transaction::Version,
    write_set::BaseStateOp,
};

#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    pub state_op: &'kv BaseStateOp,
}

impl StateUpdateRef<'_> {
    /// NOTE: the lru_info in the result is not initialized yet.
    pub fn to_result_slot(&self) -> StateSlot {
        // TODO(HotState): distinguish uninitialized lru info with a single entry (prev and next
        // are `None`).
        match self.state_op.clone() {
            BaseStateOp::Creation(value) | BaseStateOp::Modification(value) => {
                StateSlot::HotOccupied {
                    value_version: self.version,
                    value,
                    hot_since_version: self.version,
                    lru_info: LRUEntry::uninitialized(),
                }
            },
            BaseStateOp::Deletion(_) => StateSlot::HotVacant {
                hot_since_version: self.version,
                lru_info: LRUEntry::uninitialized(),
            },
            BaseStateOp::MakeHot => panic!("should not be called"),
        }
    }

    pub fn value_hash_opt(&self) -> Option<HashValue> {
        self.state_op.as_state_value_opt().map(|val| val.hash())
    }
}
