// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    state_store::{state_slot::StateSlot, state_value::StateValue},
    transaction::Version,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LRUEntry<K> {
    /// The key that is slightly newer than the current entry. `None` for the newest entry.
    pub prev: Option<K>,
    /// The key that is slightly older than the current entry. `None` for the oldest entry.
    pub next: Option<K>,
}

impl<K> LRUEntry<K> {
    pub fn uninitialized() -> Self {
        Self {
            prev: None,
            next: None,
        }
    }
}

pub trait THotStateSlot {
    type Key;

    /// Returns the key that is slightly newer in the hot state.
    fn prev(&self) -> Option<&Self::Key>;
    /// Returns the key that is slightly older in the hot state.
    fn next(&self) -> Option<&Self::Key>;

    fn set_prev(&mut self, prev: Option<Self::Key>);
    fn set_next(&mut self, next: Option<Self::Key>);
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BCSCryptoHash, CryptoHasher)]
pub struct HotStateItem {
    value: Option<StateValue>,
    hot_since_version: Version,
}

impl HotStateItem {
    pub fn new(value: Option<StateValue>, hot_since_version: Version) -> Self {
        Self {
            value,
            hot_since_version,
        }
    }
}

impl From<StateSlot> for HotStateItem {
    fn from(slot: StateSlot) -> Self {
        match slot {
            StateSlot::HotOccupied {
                value,
                hot_since_version,
                ..
            } => Self {
                value: Some(value),
                hot_since_version,
            },
            StateSlot::HotVacant {
                hot_since_version, ..
            } => Self {
                value: None,
                hot_since_version,
            },
            _ => panic!("Must be hot slot"),
        }
    }
}
