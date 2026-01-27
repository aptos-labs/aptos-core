// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    state_store::{state_slot::StateSlot, state_value::StateValue},
    transaction::Version,
};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
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

/// `HotStateValue` is what gets hashed into the hot state Merkle tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BCSCryptoHash, CryptoHasher)]
pub struct HotStateValue {
    /// `Some` means occupied and `None` means vacant.
    value: Option<StateValue>,
    hot_since_version: Version,
}

impl HotStateValue {
    pub fn new(value: Option<StateValue>, hot_since_version: Version) -> Self {
        Self {
            value,
            hot_since_version,
        }
    }

    pub fn clone_from_slot(slot: &StateSlot) -> Self {
        match slot {
            StateSlot::HotOccupied {
                value,
                hot_since_version,
                ..
            } => Self::new(Some(value.clone()), *hot_since_version),
            StateSlot::HotVacant {
                hot_since_version, ..
            } => Self::new(None, *hot_since_version),
            _ => panic!("Must be hot slot"),
        }
    }
}

/// A reference-based version of `HotStateValue` that avoids cloning `StateValue`.
/// When hashed, it produces the same hash as the equivalent `HotStateValue`.
#[derive(Serialize)]
pub struct HotStateValueRef<'a> {
    value: Option<&'a StateValue>,
    hot_since_version: Version,
}

impl<'a> HotStateValueRef<'a> {
    pub fn new(value: Option<&'a StateValue>, hot_since_version: Version) -> Self {
        Self {
            value,
            hot_since_version,
        }
    }

    pub fn from_slot(slot: &'a StateSlot) -> Self {
        match slot {
            StateSlot::HotOccupied {
                value,
                hot_since_version,
                ..
            } => Self::new(Some(value), *hot_since_version),
            StateSlot::HotVacant {
                hot_since_version, ..
            } => Self::new(None, *hot_since_version),
            _ => panic!("Must be hot slot"),
        }
    }
}

impl CryptoHash for HotStateValueRef<'_> {
    type Hasher = HotStateValueHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        bcs::serialize_into(&mut state, &self)
            .expect("BCS serialization of HotStateValueRef should not fail");
        state.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        state_store::{
            hot_state::{HotStateValue, HotStateValueRef},
            state_value::StateValue,
        },
        transaction::Version,
    };
    use aptos_crypto::hash::CryptoHash;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_hot_state_value_ref_hash(
            state_value in any::<StateValue>(),
            hot_since_version in any::<Version>(),
        ) {
            let owned = HotStateValue::new(Some(state_value.clone()), hot_since_version);
            let borrowed = HotStateValueRef::new(Some(&state_value), hot_since_version);
            assert_eq!(owned.hash(), borrowed.hash());

            let owned_none = HotStateValue::new(None, hot_since_version);
            let borrowed_none = HotStateValueRef::new(None, hot_since_version);
            assert_eq!(owned_none.hash(), borrowed_none.hash());
        }
    }
}
