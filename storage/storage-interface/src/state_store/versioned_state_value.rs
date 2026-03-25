// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    state_store::{
        hot_state::LRUEntry,
        state_key::StateKey,
        state_slot::{StateSlot, StateSlotKind},
    },
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
    pub fn to_result_slot(&self, state_key: StateKey) -> Option<StateSlot> {
        match self.state_op.clone() {
            BaseStateOp::Creation(value) | BaseStateOp::Modification(value) => {
                Some(StateSlot::new(state_key, StateSlotKind::HotOccupied {
                    value_version: self.version,
                    value,
                    hot_since_version: self.version,
                    lru_info: LRUEntry::uninitialized(),
                }))
            },
            BaseStateOp::Deletion(_) => Some(StateSlot::new(state_key, StateSlotKind::HotVacant {
                hot_since_version: self.version,
                lru_info: LRUEntry::uninitialized(),
            })),
            BaseStateOp::MakeHot => None,
        }
    }
}
