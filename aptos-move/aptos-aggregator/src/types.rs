// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::bounded_math::code_invariant_error;
use aptos_types::state_store::{state_key::StateKey, table::TableHandle};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::account_address::AccountAddress;

pub type AggregatorResult<T> = Result<T, AggregatorError>;

// TODO: Use this instead of PartialVM errors.
#[derive(Debug)]
pub enum AggregatorError {
    WrongVersionID,
}

/// Ephemeral identifier type used by aggregators V2.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatorID(u64);

impl AggregatorID {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Uniquely identifies aggregator or aggregator snapshot instances in
/// extension and possibly storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregatorVersionedID {
    // Aggregator V1 is implemented as a state item, and so can be queried by
    // the state key.
    V1(StateKey),
    // Aggregator V2 is embedded into resources, and uses ephemeral identifiers
    // which are unique per block.
    V2(AggregatorID),
}

impl AggregatorVersionedID {
    pub fn v1(handle: TableHandle, key: AccountAddress) -> Self {
        let state_key = StateKey::table_item(handle, key.to_vec());
        Self::V1(state_key)
    }

    pub fn v2(value: u64) -> Self {
        Self::V2(AggregatorID::new(value))
    }
}

impl TryFrom<AggregatorVersionedID> for StateKey {
    type Error = AggregatorError;

    fn try_from(vid: AggregatorVersionedID) -> Result<Self, Self::Error> {
        match vid {
            AggregatorVersionedID::V1(state_key) => Ok(state_key),
            AggregatorVersionedID::V2(_) => Err(AggregatorError::WrongVersionID),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorValue {
    Integer(u128),
    String(Vec<u8>),
}

impl AggregatorValue {
    pub fn into_integer_value(self) -> PartialVMResult<u128> {
        match self {
            AggregatorValue::Integer(value) => Ok(value),
            AggregatorValue::String(_) => Err(code_invariant_error(
                "Tried calling into_integer_value on String value",
            )),
        }
    }

    pub fn into_string_value(self) -> PartialVMResult<Vec<u8>> {
        match self {
            AggregatorValue::String(value) => Ok(value),
            AggregatorValue::Integer(_) => Err(code_invariant_error(
                "Tried calling into_string_value on Integer value",
            )),
        }
    }
}

// TODO this is now same as AggregatorValue, and can just be replaced, but keeping it for simplicity of comparison
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotValue {
    Integer(u128),
    String(Vec<u8>),
}

impl SnapshotValue {
    pub fn into_aggregator_value(self) -> PartialVMResult<u128> {
        match self {
            SnapshotValue::Integer(value) => Ok(value),
            SnapshotValue::String(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on String SnapshotValue",
            )),
        }
    }
}

impl TryFrom<AggregatorValue> for SnapshotValue {
    type Error = PartialVMError;

    fn try_from(value: AggregatorValue) -> PartialVMResult<SnapshotValue> {
        match value {
            AggregatorValue::Integer(v) => Ok(SnapshotValue::Integer(v)),
            AggregatorValue::String(v) => Ok(SnapshotValue::String(v)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotToStringFormula {
    Concat { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl SnapshotToStringFormula {
    pub fn apply(&self, base: u128) -> Vec<u8> {
        match self {
            SnapshotToStringFormula::Concat { prefix, suffix } => {
                let mut result = prefix.clone();
                result.extend(base.to_string().as_bytes());
                result.extend(suffix);
                result
            },
        }
    }
}
