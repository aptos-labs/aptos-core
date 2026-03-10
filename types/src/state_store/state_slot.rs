// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    state_store::{
        hot_state::{LRUEntry, THotStateSlot},
        state_key::StateKey,
        state_value::StateValue,
    },
    transaction::Version,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use StateSlot::*;

/// Represents the content of a state slot, or the lack there of, along with information indicating
/// whether the slot is present in the cold or/and hot state.
///
/// state_key: the original key, stored in non-ColdVacant variants so that JMT persistence can
///            recover the key when iterating HashValue-keyed shards.
/// value_version: non-empty value changed at this version
/// hot_since_version: the timestamp of a hot value / vacancy in the hot state, which determines
///                    the order of eviction
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateSlot {
    ColdVacant,
    HotVacant {
        state_key: StateKey,
        hot_since_version: Version,
        lru_info: LRUEntry<HashValue>,
    },
    ColdOccupied {
        state_key: StateKey,
        value_version: Version,
        value: StateValue,
    },
    HotOccupied {
        state_key: StateKey,
        value_version: Version,
        value: StateValue,
        hot_since_version: Version,
        lru_info: LRUEntry<HashValue>,
    },
}

impl StateSlot {
    fn maybe_update_cold_state(&self, min_version: Version) -> Option<Option<&StateValue>> {
        match self {
            ColdVacant => Some(None),
            HotVacant {
                hot_since_version, ..
            } => {
                if *hot_since_version >= min_version {
                    // TODO(HotState): revisit after the hot state is exclusive with the cold state
                    // Can't tell if there was a deletion to the cold state here, not much harm to
                    // issue a deletion anyway.
                    // TODO(HotState): query the base version before doing the JMT update to filter
                    //                 out "empty deletes"
                    Some(None)
                } else {
                    None
                }
            },
            ColdOccupied {
                value_version,
                value,
                ..
            }
            | HotOccupied {
                value_version,
                value,
                ..
            } => {
                if *value_version >= min_version {
                    // an update happened at or after min_version, need to update
                    Some(Some(value))
                } else {
                    // cached value from before min_version, ignore
                    None
                }
            },
        }
    }

    /// When committing speculative state to the DB, determine if to make changes to the JMT.
    /// `key_hash` is the hash of the state key (the map key in HashValue-keyed shards).
    /// For occupied variants, the StateKey is recovered from the slot itself.
    pub fn maybe_update_jmt(
        &self,
        key_hash: HashValue,
        min_version: Version,
    ) -> Option<(HashValue, Option<(HashValue, StateKey)>)> {
        let maybe_value_opt = self.maybe_update_cold_state(min_version);
        maybe_value_opt.map(|value_opt| {
            (
                key_hash,
                value_opt.map(|v| (CryptoHash::hash(v), self.expect_state_key().clone())),
            )
        })
    }

    // TODO(HotState): db returns cold slot directly
    pub fn from_db_get(state_key: StateKey, tuple_opt: Option<(Version, StateValue)>) -> Self {
        match tuple_opt {
            None => Self::ColdVacant,
            Some((value_version, value)) => Self::ColdOccupied {
                state_key,
                value_version,
                value,
            },
        }
    }

    pub fn state_key(&self) -> Option<&StateKey> {
        match self {
            ColdVacant => None,
            HotVacant { state_key, .. }
            | ColdOccupied { state_key, .. }
            | HotOccupied { state_key, .. } => Some(state_key),
        }
    }

    pub fn expect_state_key(&self) -> &StateKey {
        self.state_key()
            .expect("StateKey expected (not ColdVacant)")
    }

    pub fn into_state_value_and_version_opt(self) -> Option<(Version, StateValue)> {
        match self {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied {
                value_version,
                value,
                ..
            }
            | HotOccupied {
                value_version,
                value,
                ..
            } => Some((value_version, value)),
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
    }

    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
        match self {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
    }

    pub fn is_hot(&self) -> bool {
        !self.is_cold()
    }

    pub fn is_cold(&self) -> bool {
        match self {
            ColdVacant | ColdOccupied { .. } => true,
            HotVacant { .. } | HotOccupied { .. } => false,
        }
    }

    pub fn is_occupied(&self) -> bool {
        match self {
            ColdVacant | HotVacant { .. } => false,
            ColdOccupied { .. } | HotOccupied { .. } => true,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            ColdVacant | HotVacant { .. } => 0,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => value.size(),
        }
    }

    pub fn hot_since_version_opt(&self) -> Option<Version> {
        match self {
            ColdVacant | ColdOccupied { .. } => None,
            HotVacant {
                hot_since_version, ..
            }
            | HotOccupied {
                hot_since_version, ..
            } => Some(*hot_since_version),
        }
    }

    pub fn expect_hot_since_version(&self) -> Version {
        self.hot_since_version_opt().expect("expecting hot")
    }

    pub fn refresh(&mut self, version: Version) {
        match self {
            HotOccupied {
                hot_since_version, ..
            }
            | HotVacant {
                hot_since_version, ..
            } => *hot_since_version = version,
            _ => panic!("Should not be called on cold slots."),
        }
    }

    pub fn expect_value_version(&self) -> Version {
        match self {
            ColdVacant | HotVacant { .. } => unreachable!("expecting occupied"),
            ColdOccupied { value_version, .. } | HotOccupied { value_version, .. } => {
                *value_version
            },
        }
    }

    pub fn to_hot(self, hot_since_version: Version, state_key: StateKey) -> Self {
        match self {
            ColdOccupied {
                value_version,
                value,
                ..
            } => HotOccupied {
                state_key,
                value_version,
                value,
                hot_since_version,
                lru_info: LRUEntry::uninitialized(),
            },
            ColdVacant => HotVacant {
                state_key,
                hot_since_version,
                lru_info: LRUEntry::uninitialized(),
            },
            _ => panic!("Should not be called on hot slots."),
        }
    }

    pub fn to_cold(self) -> Self {
        match self {
            HotOccupied {
                state_key,
                value_version,
                value,
                ..
            } => ColdOccupied {
                state_key,
                value_version,
                value,
            },
            HotVacant { .. } => ColdVacant,
            _ => panic!("Should not be called on cold slots."),
        }
    }
}

impl THotStateSlot for StateSlot {
    type Key = HashValue;

    fn prev(&self) -> Option<&Self::Key> {
        match self {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.prev.as_ref(),
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn next(&self) -> Option<&Self::Key> {
        match self {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.next.as_ref(),
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn set_prev(&mut self, prev: Option<Self::Key>) {
        match self {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.prev = prev,
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn set_next(&mut self, next: Option<Self::Key>) {
        match self {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.next = next,
            _ => panic!("Should not be called on cold slots."),
        }
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for StateSlot {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        let hot_vacant =
            (any::<StateKey>(), any::<Version>()).prop_map(|(state_key, hot_since_version)| {
                HotVacant {
                    state_key,
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                }
            });
        let cold_occupied = (any::<StateKey>(), any::<Version>(), any::<StateValue>()).prop_map(
            |(state_key, value_version, value)| ColdOccupied {
                state_key,
                value_version,
                value,
            },
        );
        let hot_occupied = (any::<StateKey>(), any::<Version>(), any::<StateValue>())
            .prop_flat_map(|(state_key, value_version, value)| {
                (
                    Just(state_key),
                    Just(value_version),
                    (value_version..Version::MAX),
                    Just(value),
                )
            })
            .prop_map(
                |(state_key, value_version, hot_since_version, value)| HotOccupied {
                    state_key,
                    value_version,
                    value,
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                },
            );
        prop_oneof![
            1 => Just(ColdVacant),
            1 => hot_vacant,
            2 => cold_occupied,
            2 => hot_occupied,
        ]
        .boxed()
    }
}
