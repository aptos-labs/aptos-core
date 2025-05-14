// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    state_store::{state_slot::StateSlot, state_value::StateValue},
    transaction::Version,
};

#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    /// `None` indicates deletion.
    pub value: Option<&'kv StateValue>,
}

impl StateUpdateRef<'_> {
    /// TODO(HotState): Revisit: assuming every write op results in a hot slot
    pub fn to_hot_slot(&self) -> StateSlot {
        match self.value {
            None => StateSlot::HotVacant {
                deletion_version: Some(self.version),
                hot_since_version: self.version,
            },
            Some(value) => StateSlot::HotOccupied {
                value_version: self.version,
                value: value.clone(),
                hot_since_version: self.version,
            },
        }
    }

    pub fn value_hash_opt(&self) -> Option<HashValue> {
        self.value.as_ref().map(|val| val.hash())
    }
}
