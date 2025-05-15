// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::COUNTER;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use StateSlot::*;

/// Represents the content of a state slot, or the lack thereof, along with information indicating
/// whether the slot is present in the cold or/and hot state. This is output by the VM and will be
/// used to determine how to update hot and cold state.
///
/// - If a slot is recently read in the latest block:
///   - HotOccupied if the item exists.
///   - HotVacant if the item does not exist (so this info will be cached in hot state).
/// - If a slot is recently written to in the latest block:
///   - HotOccupied if the value is added/updated.
///   - HotVacant if the key is deleted.
/// - If a slot is not referenced recently, and needs to be evicted from hot state:
///   - ColdOccupied if it's HotOccupied before.
///   - ColdVacant if it's HotVacant before.
///
/// value_version: non-empty value changed at this version
/// hot_since_version: the timestamp of a hot value / vacancy in the hot state, which determines
///                    the order of eviction
#[derive(Clone, Debug)]
pub enum StateSlot {
    ColdVacant,
    HotVacant {
        /// None - unknown, from a DB read
        /// Some - from a WriteOp::Deletion()
        deletion_version: Option<Version>,
        hot_since_version: Version,
    },
    ColdOccupied {
        value_version: Version,
        value: StateValue,
    },
    HotOccupied {
        value_version: Version,
        value: StateValue,
        hot_since_version: Version,
    },
}

impl StateSlot {
    // TODO(HotState): revisit after refresh determined in the VM
    ///  Called when an item is read, determining if to write the hot state:
    ///     if cold, output hot slot
    ///     if hot and old: output hot slot with refreshed hot_since_version
    ///     if hot and relatively new, output None indicating no need to alter the hot state
    pub fn maybe_make_hot_or_refresh(
        &self,
        version: Version,
        refresh_interval_versions: usize,
    ) -> Option<Self> {
        match self {
            ColdVacant => {
                COUNTER.inc_with(&["memorized_read_new_hot_non_existent"]);
                Some(HotVacant {
                    deletion_version: None,
                    hot_since_version: version,
                })
            },
            HotVacant {
                deletion_version,
                hot_since_version,
            } => {
                if Self::should_refresh(version, refresh_interval_versions, hot_since_version) {
                    COUNTER.inc_with(&["memorized_read_refreshed_hot_vacant"]);
                    Some(HotVacant {
                        deletion_version: *deletion_version,
                        hot_since_version: version,
                    })
                } else {
                    None
                }
            },
            ColdOccupied {
                value_version,
                value,
            } => {
                COUNTER.inc_with(&["memorized_read_new_hot"]);
                Some(HotOccupied {
                    value_version: *value_version,
                    value: value.clone(),
                    hot_since_version: version,
                })
            },
            HotOccupied {
                value_version,
                value,
                hot_since_version,
            } => {
                if Self::should_refresh(version, refresh_interval_versions, hot_since_version) {
                    COUNTER.inc_with(&["memorized_read_refreshed_hot"]);
                    Some(HotOccupied {
                        value_version: *value_version,
                        value: value.clone(),
                        hot_since_version: version,
                    })
                } else {
                    COUNTER.inc_with(&["memorized_read_still_hot"]);
                    None
                }
            },
        } // end match
    }

    fn should_refresh(
        version: Version,
        refresh_interval_versions: usize,
        hot_since_version: &Version,
    ) -> bool {
        // e.g. if hot since version 0, refresh interval is 10 versions,
        //      and it gets read at every version, refresh at version 10, 20, ...
        hot_since_version + refresh_interval_versions as u64 <= version
    }

    fn maybe_update_cold_state(&self, min_version: Version) -> Option<Option<&StateValue>> {
        match self {
            ColdVacant => Some(None),
            HotVacant {
                deletion_version,
                hot_since_version: _,
            } => deletion_version
                .map(|ver| ver >= min_version)
                .unwrap_or(false)
                .then_some(None),
            ColdOccupied {
                value_version,
                value,
            }
            | HotOccupied {
                value_version,
                value,
                hot_since_version: _,
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
        key: StateKey,
        min_version: Version,
    ) -> Option<(HashValue, Option<(HashValue, StateKey)>)> {
        let maybe_value_opt = self.maybe_update_cold_state(min_version);
        maybe_value_opt.map(|value_opt| {
            (
                CryptoHash::hash(&key),
                value_opt.map(|v| (CryptoHash::hash(v), key)),
            )
        })
    }

    // TODO(HotState): db returns cold slot directly
    pub fn from_db_get(tuple_opt: Option<(Version, StateValue)>) -> Self {
        match tuple_opt {
            None => Self::ColdVacant,
            Some((value_version, value)) => Self::ColdOccupied {
                value_version,
                value,
            },
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
    }

    pub fn to_state_value_ref_opt(&self) -> Option<&StateValue> {
        match self {
            ColdVacant | HotVacant { .. } => None,
            ColdOccupied { value, .. } | HotOccupied { value, .. } => Some(value),
        }
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

    pub fn expect_hot_since_version(&self) -> Version {
        match self {
            ColdVacant | ColdOccupied { .. } => unreachable!("expecting hot"),
            HotVacant {
                hot_since_version, ..
            }
            | HotOccupied {
                hot_since_version, ..
            } => *hot_since_version,
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
}
