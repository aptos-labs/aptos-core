// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{state_store::state_value::StateValue, transaction::Version};

#[derive(Clone, Debug)]
pub struct StateWrite {
    /// The version where the key got updated (incl. deletion).
    pub version: Version,
    /// `None` indicates deletion.
    pub value: Option<StateValue>,
}

impl StateWrite {
    pub fn to_state_value_with_version(&self) -> StateValueWithVersionOpt {
        use StateValueWithVersionOpt::*;

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
pub enum StateValueWithVersionOpt {
    /// Not indicating if the value ever existed and deleted.
    NonExistent,
    /// A creation or modification.
    Value { version: Version, value: StateValue },
}

impl StateValueWithVersionOpt {
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
