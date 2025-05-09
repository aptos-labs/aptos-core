// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::COUNTER;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};

/// State slot content with the version for the last change to it.
///
/// | value.is_existing() | version | meaning                                         |
/// | ------------------- | ------- | ----------------------------------------------- |
/// | true                | Some    | existing and last change version known          |
/// | true                | None    | invalid -- non-empty slot version must be known |
/// | false               | Some    | Deletion at known version                       |
/// | false               | None    | Non-existent, unclear if ever existed           |
#[derive(Clone, Debug)]
pub struct DbStateUpdate {
    /// TODO(HotState): Revisit: a mere move between the hot and cold state tiers doesn't change
    ///                 the version.
    pub version: Some(Version),
    pub value: StateValue,
}

impl DbStateUpdate {
    pub fn to_jmt_update_opt(
        &self,
        key: StateKey,
        min_version: Version,
    ) -> Option<(HashValue, Option<(HashValue, StateKey)>)> {
        // Items from < min_version are cached old items.
        if self.version < min_version {
            return None;
        }

        match &self.value {
            None => {
                // HotNonExistent is not explicitly evicted for now, so this must be a real delete
                // on the jmt
                Some((CryptoHash::hash(&key), None))
            },
            Some(db_val) => {
                if db_val.is_hot_non_existent() {
                    // not persisting HotNoneExistent for now
                    None
                } else {
                    Some((
                        CryptoHash::hash(&key),
                        Some((CryptoHash::hash(db_val.expect_state_value()), key)),
                    ))
                }
            },
        }
    }

    pub fn expect_non_delete(&self) -> &StateValue {
        self.value.as_ref().expect("Unexpected deletion.")
    }
}

#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    /// `None` indicates deletion.
    /// TODO(aldenhu): Maybe upgrade to make DbStateValue here already,  but for now this is raw
    ///                VM output that doesn't involve hot state manipulation.
    pub value: Option<&'kv StateValue>,
}

impl<'kv> StateUpdateRef<'kv> {
    pub fn to_dbs_tate_update(&self, access_time_secs: u32) -> DbStateUpdate {
        DbStateUpdate {
            version: self.version,
            value: self
                .value
                .cloned()
                .map(|val| val.with_hot_since_usecs(access_time_secs)),
        }
    }

    pub fn value_hash_opt(&self) -> Option<HashValue> {
        self.value.as_ref().map(|val| val.hash())
    }
}

/*
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
                value: Some(value.with_hot_since_usecs(0)),
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
                    // TODO(HotState):
                    // Dummy creation version
                    version: 0,
                    value: Some(StateValue::new_hot_non_existent(access_time_secs)),
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
*/