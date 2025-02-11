// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{state_store::state_value::StateValue, transaction::Version};

#[derive(Clone, Debug)]
pub struct StateUpdate {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    /// `None` indicates deletion.
    pub value: Option<StateValue>,
}

impl StateUpdate {
    pub fn to_state_value_with_version(&self, access_time_secs: u32) -> StateCacheEntry {
        use StateCacheEntry::*;

        match &self.value {
            None => NonExistent { access_time_secs },
            Some(value) => Value {
                access_time_secs,
                version: self.version,
                value: value.clone(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct StateUpdateRef<'kv> {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    /// `None` indicates deletion.
    pub value: Option<&'kv StateValue>,
}

impl<'kv> StateUpdateRef<'kv> {
    pub fn cloned(&self) -> StateUpdate {
        StateUpdate {
            version: self.version,
            value: self.value.cloned(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum StateCacheEntry {
    /// NOT indicating whether the value never existed or deleted.
    NonExistent { access_time_secs: u32 },
    /// A creation or modification.
    Value {
        access_time_secs: u32,
        version: Version,
        value: StateValue,
    },
}

impl StateCacheEntry {
    // TODO(aldenhu): update DbReader interface to return this type directly.
    pub fn from_db_tuple_opt(
        tuple_opt: Option<(Version, StateValue)>,
        access_time_secs: u32,
    ) -> Self {
        match tuple_opt {
            None => Self::NonExistent { access_time_secs },
            Some((version, value)) => Self::Value {
                access_time_secs,
                version,
                value,
            },
        }
    }

    pub fn from_state_update_ref(state_update_ref: &StateUpdateRef, access_time_secs: u32) -> Self {
        match state_update_ref.value {
            None => Self::NonExistent { access_time_secs },
            Some(value) => Self::Value {
                access_time_secs,
                version: state_update_ref.version,
                value: value.clone(),
            },
        }
    }

    pub fn to_state_value_opt(&self) -> Option<StateValue> {
        self.state_value_ref_opt().cloned()
    }

    pub fn state_value_ref_opt(&self) -> Option<&StateValue> {
        match self {
            Self::NonExistent { .. } => None,
            Self::Value { value, .. } => Some(value),
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self {
            Self::NonExistent { .. } => None,
            Self::Value { value, .. } => Some(value),
        }
    }
}
