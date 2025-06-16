// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    state_store::state_slot::StateSlot, transaction::Version, write_set::BaseStateOp,
};

#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    pub state_op: &'kv BaseStateOp,
}

impl StateUpdateRef<'_> {
    pub fn to_result_slot(&self) -> StateSlot {
        match self.state_op.clone() {
            BaseStateOp::Creation(value) | BaseStateOp::Modification(value) => {
                StateSlot::HotOccupied {
                    value_version: self.version,
                    value,
                    hot_since_version: self.version,
                }
            },
            BaseStateOp::Deletion(_) => StateSlot::HotVacant {
                hot_since_version: self.version,
            },
            BaseStateOp::MakeHot { prev_slot } => match prev_slot {
                StateSlot::ColdVacant => StateSlot::HotVacant {
                    hot_since_version: self.version,
                },
                StateSlot::HotVacant {
                    hot_since_version: _,
                } => StateSlot::HotVacant {
                    hot_since_version: self.version,
                },
                StateSlot::ColdOccupied {
                    value_version,
                    value,
                }
                | StateSlot::HotOccupied {
                    value_version,
                    value,
                    hot_since_version: _,
                } => StateSlot::HotOccupied {
                    value_version,
                    value,
                    hot_since_version: self.version,
                },
            },
        }
    }

    pub fn value_hash_opt(&self) -> Option<HashValue> {
        self.state_op.as_state_value_opt().map(|val| val.hash())
    }
}
