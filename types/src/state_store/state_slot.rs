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
#[derive(Clone, Debug, Derivative, Eq, PartialEq)]
pub enum StateSlot {
    ColdVacant,
    HotVacant {
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
    fn maybe_update_cold_state(&self, min_version: Version) -> Option<Option<&StateValue>> {
        match self {
            ColdVacant => Some(None),
            HotVacant { hot_since_version } => {
                if *hot_since_version >= min_version {
                    // TODO(HotState): revisit after the hot state is exclusive with the cold state
                    // Can't tell if there was a deletion to the cold state here, not much harm to
                    // issue a deletion anyway.
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

    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
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
