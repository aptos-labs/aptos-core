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
    pub fn to_state_value_with_version(&self) -> StateCacheEntry {
        use StateCacheEntry::*;

        match &self.value {
            None => NonExistent,
            Some(value) => Value {
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
    /// Not indicating if the value ever existed and deleted.
    NonExistent,
    /// A creation or modification.
    Value { version: Version, value: StateValue },
}

impl StateCacheEntry {
    // TODO(aldenhu): update DbReader interface to return this type directly.
    pub fn from_tuple_opt(tuple_opt: Option<(Version, StateValue)>) -> Self {
        match tuple_opt {
            None => Self::NonExistent,
            Some((version, value)) => Self::Value { version, value },
        }
    }

    pub fn from_state_write_ref(version: Version, value_opt: Option<&StateValue>) -> Self {
        match value_opt {
            None => Self::NonExistent,
            Some(value) => Self::Value {
                version,
                value: value.clone(),
            },
        }
    }

    pub fn from_state_update_ref(state_update_ref: &StateUpdateRef) -> Self {
        match state_update_ref.value {
            None => Self::NonExistent,
            Some(value) => Self::Value {
                version: state_update_ref.version,
                value: value.clone(),
            },
        }
    }

    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
        match self {
            Self::NonExistent => None,
            Self::Value { value, .. } => Some(value),
        }
    }

    pub fn to_state_value_opt(&self) -> Option<StateValue> {
        match self {
            Self::NonExistent => None,
            Self::Value { value, .. } => Some(value.clone()),
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match self {
            Self::NonExistent => None,
            Self::Value { value, .. } => Some(value),
        }
    }
}
