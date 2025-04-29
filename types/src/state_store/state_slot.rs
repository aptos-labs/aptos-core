// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use derivative::Derivative;
use StateSlot::*;

/// Represents the content of a state slot, or the lack there of, along with information indicating
/// whether the slot is present in the cold or/and hot state.
///
/// value_version: non-empty value changed at this version
/// hot_since_version: the timestamp of a hot value / vacancy in the hot state, which determines
///                    the order of eviction
#[derive(Clone, Debug, Derivative, Eq)]
#[derivative(PartialEq)]
pub enum StateSlot {
    ColdVacant,
    HotVacant {
        /// None - unknown, from a DB read
        /// Some - from a WriteOp::Deletion()
        #[derivative(PartialEq = "ignore")]
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
                // COUNTER.inc_with(&["memorized_read_new_hot_non_existent"]);
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
                    // COUNTER.inc_with(&["memorized_read_refreshed_hot_vacant"]);
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
                // COUNTER.inc_with(&["memorized_read_new_hot"]);
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
                    // COUNTER.inc_with(&["memorized_read_refreshed_hot"]);
                    Some(HotOccupied {
                        value_version: *value_version,
                        value: value.clone(),
                        hot_since_version: version,
                    })
                } else {
                    // COUNTER.inc_with(&["memorized_read_still_hot"]);
                    None
                }
            },
        } // end match
    }

    pub fn into_hot(self, version: Version) -> Self {
        match self {
            ColdVacant => HotVacant {
                deletion_version: None,
                hot_since_version: version,
            },
            HotVacant {
                deletion_version,
                hot_since_version: _,
            } => HotVacant {
                deletion_version,
                hot_since_version: version,
            },
            ColdOccupied {
                value_version,
                value,
            }
            | HotOccupied {
                value_version,
                value,
                hot_since_version: _,
            } => HotOccupied {
                value_version,
                value,
                hot_since_version: version,
            },
        }
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

    pub fn expect_value_version(&self) -> Version {
        match self {
            ColdVacant | HotVacant { .. } => unreachable!("expecting occupied"),
            ColdOccupied { value_version, .. } | HotOccupied { value_version, .. } => {
                *value_version
            },
        }
    }
}
