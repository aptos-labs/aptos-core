// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
    pub fn to_result_slot(&self) -> Option<StateSlot> {
        match self.state_op.clone() {
            BaseStateOp::Creation(value) | BaseStateOp::Modification(value) => {
                Some(StateSlot::HotOccupied {
                    value_version: self.version,
                    value,
                    hot_since_version: self.version,
                    lru_info: LRUEntry::uninitialized(),
                })
            },
            BaseStateOp::Deletion(_) => Some(StateSlot::HotVacant {
                hot_since_version: self.version,
                lru_info: LRUEntry::uninitialized(),
            }),
            BaseStateOp::MakeHot => None,
        }
    }
}
