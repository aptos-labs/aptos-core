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
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use StateSlotKind::*;

/// Represents the content of a state slot along with its key and information about
/// whether the slot is present in the cold or/and hot state.
///
/// `state_key` is `None` for slots from the hot state base, which keeps only the key hash to save
/// memory. That includes slots loaded from the hot state KV DB.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSlot {
    state_key: Option<StateKey>,
    kind: StateSlotKind,
}

/// What the cold state holds for a hot slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum ColdCopy {
    /// Cold has the hot value, at `value_version` if occupied.
    InSync,
    /// Cold doesn't have the hot value yet.
    Behind {
        /// Version of the cold KV row with the key's current cold value. `None` if cold has no
        /// value.
        version: Option<Version>,
    },
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
        cold_copy: ColdCopy,
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
        cold_copy: ColdCopy,
        lru_info: LRUEntry<HashValue>,
    },
}

impl StateSlotKind {
    /// Returns a copy of this kind with the LRU info replaced. Panics on cold variants.
    pub fn with_lru_info(self, lru_info: LRUEntry<HashValue>) -> Self {
        match self {
            StateSlotKind::HotOccupied {
                value_version,
                value,
                hot_since_version,
                cold_copy,
                ..
            } => StateSlotKind::HotOccupied {
                value_version,
                value,
                hot_since_version,
                cold_copy,
                lru_info,
            },
            StateSlotKind::HotVacant {
                hot_since_version,
                cold_copy,
                ..
            } => StateSlotKind::HotVacant {
                hot_since_version,
                cold_copy,
                lru_info,
            },
            _ => panic!("with_lru_info called on cold variant"),
        }
    }
}

impl StateSlot {
    pub fn new(state_key: StateKey, kind: StateSlotKind) -> Self {
        Self {
            state_key: Some(state_key),
            kind,
        }
    }

    /// Creates a `StateSlot` without a `StateKey`. Used when loading from the hot state
    /// KV DB, which only stores the key hash.
    pub fn new_without_state_key(kind: StateSlotKind) -> Self {
        Self {
            state_key: None,
            kind,
        }
    }

    pub fn state_key(&self) -> Option<&StateKey> {
        self.state_key.as_ref()
    }

    pub fn set_state_key(&mut self, state_key: StateKey) {
        self.state_key = Some(state_key);
    }

    pub fn clear_state_key(&mut self) {
        self.state_key = None;
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

    /// JMT-pass inclusion filter. Returns `true` for slots that
    /// should contribute a JMT-pass entry; `false` for slots that
    /// haven't changed since `min_version` (only LRU pointer updates,
    /// stale hot evictions, etc.). Used by the shared pipeline-level
    /// snapshot committer loop alongside the generic
    /// `leaf_entry_to_jmt_update` extractor.
    pub fn passes_jmt_filter(&self, min_version: Version) -> bool {
        self.maybe_update_cold_state(min_version).is_some()
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
        Self {
            state_key: Some(state_key),
            kind,
        }
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

    pub fn expect_cold_copy(&self) -> ColdCopy {
        match &self.kind {
            HotVacant { cold_copy, .. } | HotOccupied { cold_copy, .. } => *cold_copy,
            ColdVacant | ColdOccupied { .. } => panic!("expecting hot"),
        }
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
                cold_copy: ColdCopy::InSync,
                lru_info: LRUEntry::uninitialized(),
            },
            ColdVacant => HotVacant {
                hot_since_version,
                cold_copy: ColdCopy::InSync,
                lru_info: LRUEntry::uninitialized(),
            },
            _ => panic!("Should not be called on hot slots."),
        };
        Self {
            state_key: self.state_key,
            kind,
        }
    }

    /// If the cold copy is behind, the hot value must be written to cold at `eviction_version`,
    /// as a cold slot's `value_version` is the version of its cold KV row.
    pub fn to_cold(self, eviction_version: Version) -> Self {
        let kind = match self.kind {
            HotOccupied {
                value_version,
                value,
                cold_copy,
                ..
            } => ColdOccupied {
                value_version: match cold_copy {
                    ColdCopy::InSync => value_version,
                    ColdCopy::Behind { .. } => eviction_version,
                },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hot_occupied(value_version: Version, cold_copy: ColdCopy) -> StateSlot {
        StateSlot::new(StateKey::raw(b"k"), HotOccupied {
            value_version,
            value: StateValue::new_legacy(b"v".to_vec().into()),
            hot_since_version: 20,
            cold_copy,
            lru_info: LRUEntry::uninitialized(),
        })
    }

    fn hot_vacant(cold_copy: ColdCopy) -> StateSlot {
        StateSlot::new(StateKey::raw(b"k"), HotVacant {
            hot_since_version: 20,
            cold_copy,
            lru_info: LRUEntry::uninitialized(),
        })
    }

    #[test]
    fn test_to_hot_is_in_sync() {
        let occupied = StateSlot::from_db_get(
            StateKey::raw(b"k"),
            Some((10, StateValue::new_legacy(b"v".to_vec().into()))),
        )
        .to_hot(20);
        assert_eq!(occupied.expect_cold_copy(), ColdCopy::InSync);
        assert_eq!(occupied.expect_value_version(), 10);

        let vacant = StateSlot::from_db_get(StateKey::raw(b"k"), None).to_hot(20);
        assert_eq!(vacant.expect_cold_copy(), ColdCopy::InSync);
    }

    #[test]
    fn test_refresh_keeps_cold_copy() {
        let behind = ColdCopy::Behind { version: Some(5) };
        let mut slot = hot_occupied(10, behind);
        slot.refresh(30);
        assert_eq!(slot.expect_cold_copy(), behind);
    }

    #[test]
    fn test_to_cold_in_sync_keeps_value_version() {
        let slot = hot_occupied(10, ColdCopy::InSync).to_cold(30);
        assert_eq!(slot.kind(), &ColdOccupied {
            value_version: 10,
            value: StateValue::new_legacy(b"v".to_vec().into()),
        });
        assert_eq!(hot_vacant(ColdCopy::InSync).to_cold(30).kind(), &ColdVacant);
    }

    #[test]
    fn test_to_cold_behind_takes_eviction_version() {
        for version in [None, Some(5)] {
            let slot = hot_occupied(10, ColdCopy::Behind { version }).to_cold(30);
            assert_eq!(slot.kind(), &ColdOccupied {
                value_version: 30,
                value: StateValue::new_legacy(b"v".to_vec().into()),
            });
            assert_eq!(
                hot_vacant(ColdCopy::Behind { version }).to_cold(30).kind(),
                &ColdVacant
            );
        }
    }
}
