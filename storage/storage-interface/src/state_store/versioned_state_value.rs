// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::COUNTER;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StateSlot, StateValue},
    },
    transaction::Version,
};

// FIXME(aldenhu): rename to StateUpdate
/// State slot content with the version for the last change to it.
///
/// | slot.occupied() | version | meaning                                        |
/// | --------------- | ------- | ---------------------------------------------- |
/// | true            | Some    | existing and last change version known         |
/// | true            | None    | invalid -- occupied slot version must be known |
/// | false           | Some    | Deletion at known version                      |
/// | false           | None    | Non-existent, unclear if ever existed          |
#[derive(Clone, Debug)]
pub struct DbStateUpdate {
    /// TODO(HotState): Revisit: a mere move between the hot and cold state tiers doesn't change
    ///                 the version.
    pub version: Option<Version>,
    pub slot: StateSlot,
}

impl DbStateUpdate {
    pub fn get_ref(&self) -> StateUpdateRef {
        StateUpdateRef {
            version: self.version,
            slot: &self.slot,
        }
    }

    pub fn to_state_value_opt(&self) -> Option<&StateValue> {
        self.slot.to_state_value_opt()
    }

    pub fn maybe_to_cold_state_update(&self, chunk_first_version: Version) -> Option<ColdStateUpdateRef> {
        ColdStateUpdateRef::maybe_new_from_state_update(self.version, chunk_first_version, &self.slot)
    }
}

/// Borrowed version of StateUpdate.
#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    pub version: Option<Version>,
    pub slot: &'kv StateSlot,
}

impl<'kv> StateUpdateRef<'kv> {
    // FIXME(aldenhu): rename to to_owned or to_state_update
    pub fn to_db_state_update(&self, access_time_secs: u32) -> DbStateUpdate {
        DbStateUpdate {
            version: self.version,
            slot: self.slot.clone(),
        }
    }

    pub fn value_hash_opt(&self) -> Option<HashValue> {
        self.slot.to_state_value_opt().map(CryptoHash::hash)
    }

    pub fn maybe_to_cold_state_update(&self, chunk_first_version: Version) -> Option<ColdStateUpdateRef<'kv>> {
        ColdStateUpdateRef::maybe_new_from_state_update(self.version, chunk_first_version, self.slot)
    }
}

pub struct ColdStateUpdateRef<'kv> {
    value: Option<&'kv StateValue>,
}

impl<'kv> ColdStateUpdateRef<'kv> {
    pub fn maybe_new_from_state_update(
        version: Option<Version>,
        chunk_first_version: Version,
        slot: &StateSlot,
    ) -> Option<Self> {
        if version.is_none() {
            // Cached empty slot
            return None
        }
        let version = version.unwrap();

        if version < chunk_first_version {
            // Cached old slot that's not updated in the chunk being executed or applied.
            return None;
        }

        // TODO(HotState): Revisit when the hot state is exclusive to the cold state.
        // Content changed in the chunk, should be updated in the cold state
        Some(Self {
            value: slot.to_state_value_opt()
        })
    }

    pub fn to_jmt_update(
        &self,
        key: &StateKey,
    ) -> (HashValue, Option<(HashValue, StateKey)>) {
        let key_hash = CryptoHash::hash(key);
        let value_hash_and_key = self.value.map(|value| {
            (CryptoHash::hash(value), key.clone())
        });

        (key_hash, value_hash_and_key)
    }

    pub fn is_delete(&self) -> bool {
        self.value.is_none()
    }

    pub fn value_size(&self) -> usize {
        self.value.map_or(0, StateValue::size)
    }
}

#[derive(Clone, Debug)]
pub enum MemorizedStateRead {
    /// Underlying storage doesn't have an entry for this state key.
    NonExistent,
    /// A state update at a known version
    StateUpdate(DbStateUpdate),
}

impl MemorizedStateRead {
    pub fn from_db_get(tuple_opt: Option<(Version, StateValue)>) -> Self {
        match tuple_opt {
            None => Self::NonExistent,
            Some((version, value)) => Self::StateUpdate(DbStateUpdate {
                version,
                // N.B. Item will end up in hot state with refreshed access time down the stack.
                value: Some(value.into_db_state_value(0)),
            }),
        }
    }

    pub fn from_speculative_state(db_update: DbStateUpdate) -> Self {
        Self::StateUpdate(db_update)
    }

    pub fn from_hot_state_hit(db_update: DbStateUpdate) -> Self {
        Self::StateUpdate(db_update)
    }

    /// TODO(aldenhu): Remove. Use only in a context where the access time doesn't matter
    pub fn dummy_from_state_update_ref(state_update_ref: &StateUpdateRef) -> Self {
        Self::StateUpdate(state_update_ref.to_db_state_update(0))
    }

    pub fn to_state_value_opt(&self) -> Option<StateValue> {
        self.to_state_value_ref_opt().cloned()
    }

    pub fn to_state_value_ref_opt(&self) -> Option<&StateValue> {
        match self {
            Self::NonExistent => None,
            Self::StateUpdate(DbStateUpdate { version: _, value }) => value
                .as_ref()
                .and_then(|db_val| db_val.to_state_value_opt()),
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self {
            Self::NonExistent => None,
            Self::StateUpdate(DbStateUpdate { version: _, value }) => {
                value.and_then(|db_val| db_val.into_state_value_opt())
            },
        }
    }

    pub fn to_hot_state_refresh(
        &self,
        access_time_secs: u32,
        refresh_internal_secs: u32,
    ) -> Option<DbStateUpdate> {
        match self {
            MemorizedStateRead::NonExistent => {
                COUNTER.inc_with(&["memorized_read_new_hot_non_existent"]);
                Some(DbStateUpdate {
                    // Dummy creation version
                    version: 0,
                    value: Some(StateSlot::new_hot_non_existent(access_time_secs)),
                })
            },
            MemorizedStateRead::StateUpdate(DbStateUpdate { version, value }) => {
                match value {
                    None => {
                        // a deletion from speculative state, no need to refresh
                        COUNTER.inc_with(&["memorized_read_speculative_delete"]);
                        None
                    },
                    Some(db_value) => {
                        let old_ts = db_value.access_time_secs();
                        if old_ts + refresh_internal_secs < access_time_secs {
                            if old_ts == 0 {
                                // comes from DB read
                                COUNTER.inc_with(&["memorized_read_new_hot"]);
                            } else {
                                COUNTER.inc_with(&["memorized_read_refreshed_hot"]);
                            }
                            Some(DbStateUpdate {
                                version: *version,
                                value: Some(
                                    db_value.clone().with_access_time_secs(access_time_secs),
                                ),
                            })
                        } else {
                            COUNTER.inc_with(&["memorized_read_still_hot"]);
                            None
                        } // end if-else
                    },
                }
            }, // end ::Value
        } // end match
    }
}
