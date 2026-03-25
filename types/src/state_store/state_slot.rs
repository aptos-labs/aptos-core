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
use StateSlotKind::*;

/// Represents the content of a state slot along with its key and information about
/// whether the slot is present in the cold or/and hot state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSlot {
    state_key: StateKey,
    kind: StateSlotKind,
}

/// The variant of a state slot.
///
/// value_version: non-empty value changed at this version
/// hot_since_version: the timestamp of a hot value / vacancy in the hot state, which determines
///                    the order of eviction
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateSlotKind {
    ColdVacant,
    HotVacant {
        hot_since_version: Version,
        lru_info: LRUEntry<HashValue>,
    },
    ColdOccupied {
        value_version: Version,
        value: StateValue,
    },
    HotOccupied {
        value_version: Version,
        value: StateValue,
        hot_since_version: Version,
        lru_info: LRUEntry<HashValue>,
    },
}

impl StateSlot {
    pub fn new(state_key: StateKey, kind: StateSlotKind) -> Self {
        Self { state_key, kind }
    }

    pub fn state_key(&self) -> &StateKey {
        &self.state_key
    }

    pub fn kind(&self) -> &StateSlotKind {
        &self.kind
    }

    fn maybe_update_cold_state(&self, min_version: Version) -> Option<Option<&StateValue>> {
        match &self.kind {
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
    pub fn maybe_update_jmt(
        &self,
        min_version: Version,
    ) -> Option<(HashValue, Option<(HashValue, StateKey)>)> {
        self.maybe_update_cold_state(min_version).map(|value_opt| {
            (
                *self.state_key.crypto_hash_ref(),
                value_opt.map(|v| (CryptoHash::hash(v), self.state_key.clone())),
            )
        })
    }

    // TODO(HotState): db returns cold slot directly
    pub fn from_db_get(state_key: StateKey, tuple_opt: Option<(Version, StateValue)>) -> Self {
        let kind = match tuple_opt {
            None => ColdVacant,
            Some((value_version, value)) => ColdOccupied {
                value_version,
                value,
            },
        };
        Self { state_key, kind }
    }

    pub fn into_state_value_and_version_opt(self) -> Option<(Version, StateValue)> {
        match self.kind {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied {
                value_version,
                value,
            }
            | HotOccupied {
                value_version,
                value,
                ..
            } => Some((value_version, value)),
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self.kind {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
    }

    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
        match &self.kind {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
    }

    pub fn is_hot(&self) -> bool {
        !self.is_cold()
    }

    pub fn is_cold(&self) -> bool {
        matches!(self.kind, ColdVacant | ColdOccupied { .. })
    }

    pub fn is_occupied(&self) -> bool {
        matches!(self.kind, ColdOccupied { .. } | HotOccupied { .. })
    }

    pub fn size(&self) -> usize {
        match &self.kind {
            ColdVacant | HotVacant { .. } => 0,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => value.size(),
        }
    }

    pub fn hot_since_version_opt(&self) -> Option<Version> {
        match &self.kind {
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
        match &mut self.kind {
            HotOccupied {
                hot_since_version, ..
            }
            | HotVacant {
                hot_since_version, ..
            } => *hot_since_version = version,
            _ => panic!("Should not be called on cold slots."),
        }
    }

    pub fn value_version_opt(&self) -> Option<Version> {
        match &self.kind {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value_version, .. } | HotOccupied { value_version, .. } => {
                Some(*value_version)
            },
        }
    }

    pub fn expect_value_version(&self) -> Version {
        self.value_version_opt().expect("expecting occupied")
    }

    pub fn to_hot(self, hot_since_version: Version) -> Self {
        let kind = match self.kind {
            ColdOccupied {
                value_version,
                value,
            } => HotOccupied {
                value_version,
                value,
                hot_since_version,
                lru_info: LRUEntry::uninitialized(),
            },
            ColdVacant => HotVacant {
                hot_since_version,
                lru_info: LRUEntry::uninitialized(),
            },
            _ => panic!("Should not be called on hot slots."),
        };
        Self {
            state_key: self.state_key,
            kind,
        }
    }

    pub fn to_cold(self) -> Self {
        let kind = match self.kind {
            HotOccupied {
                value_version,
                value,
                ..
            } => ColdOccupied {
                value_version,
                value,
            },
            HotVacant { .. } => ColdVacant,
            _ => panic!("Should not be called on cold slots."),
        };
        Self {
            state_key: self.state_key,
            kind,
        }
    }
}

impl THotStateSlot for StateSlot {
    type Key = HashValue;

    fn prev(&self) -> Option<&Self::Key> {
        match &self.kind {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.prev.as_ref(),
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn next(&self) -> Option<&Self::Key> {
        match &self.kind {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.next.as_ref(),
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn set_prev(&mut self, prev: Option<Self::Key>) {
        match &mut self.kind {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.prev = prev,
            _ => panic!("Should not be called on cold slots."),
        }
    }

    fn set_next(&mut self, next: Option<Self::Key>) {
        match &mut self.kind {
            HotOccupied { lru_info, .. } | HotVacant { lru_info, .. } => lru_info.next = next,
            _ => panic!("Should not be called on cold slots."),
        }
    }
}
