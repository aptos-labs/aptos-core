// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::code_invariant_error,
    utils::{
        bytes_to_string, from_utf8_bytes, is_string_layout, string_to_bytes, to_utf8_bytes,
        u128_to_u64,
    },
};
use aptos_types::state_store::{state_key::StateKey, table::TableHandle};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::values::{Struct, Value};

/// Types which implement this trait can be converted to a Move value.
pub trait TryIntoMoveValue: Sized {
    type Error: std::fmt::Display;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error>;
}

/// Types which implement this trait can be constructed from a Move value.
pub trait TryFromMoveValue: Sized {
    // Allows to pass extra information from the caller.
    type Hint;
    type Error: std::fmt::Display;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<Self, Self::Error>;
}

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

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

// Used for ID generation from u32/u64 counters.
impl From<u64> for AggregatorID {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl TryIntoMoveValue for AggregatorID {
    type Error = PartialVMError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.0),
            MoveTypeLayout::U128 => Value::u128(self.0 as u128),
            layout if is_string_layout(layout) => bytes_to_string(to_utf8_bytes(self.0)),
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
                        "Failed to convert {:?} into a Move value with {} layout",
                        self, layout
                    )),
                )
            },
        })
    }
}

impl TryFromMoveValue for AggregatorID {
    type Error = PartialVMError;
    type Hint = ();

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        _hint: &Self::Hint,
    ) -> Result<Self, Self::Error> {
        Ok(Self::new(match layout {
            MoveTypeLayout::U64 => value.value_as::<u64>()?,
            MoveTypeLayout::U128 => u128_to_u64(value.value_as::<u128>()?)?,
            layout if is_string_layout(layout) => {
                let bytes = string_to_bytes(value.value_as::<Struct>()?)?;
                from_utf8_bytes(bytes)?
            },
            // We use value to ID conversion in serialization.
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
                        "Failed to convert a Move value with {} layout into an identifier",
                        layout
                    )),
                )
            },
        }))
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
    Aggregator(u128),
    Snapshot(u128),
    Derived(Vec<u8>),
}

impl AggregatorValue {
    pub fn into_aggregator_value(self) -> PartialVMResult<u128> {
        match self {
            AggregatorValue::Aggregator(value) => Ok(value),
            AggregatorValue::Snapshot(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on Snapshot value",
            )),
            AggregatorValue::Derived(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on String SnapshotValue",
            )),
        }
    }

    pub fn into_snapshot_value(self) -> PartialVMResult<u128> {
        match self {
            AggregatorValue::Snapshot(value) => Ok(value),
            AggregatorValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling into_snapshot_value on Aggregator value",
            )),
            AggregatorValue::Derived(_) => Err(code_invariant_error(
                "Tried calling into_snapshot_value on String SnapshotValue",
            )),
        }
    }

    pub fn into_derived_value(self) -> PartialVMResult<Vec<u8>> {
        match self {
            AggregatorValue::Derived(value) => Ok(value),
            AggregatorValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling into_derived_value on Aggregator value",
            )),
            AggregatorValue::Snapshot(_) => Err(code_invariant_error(
                "Tried calling into_derived_value on Snapshot value",
            )),
        }
    }
}

impl TryIntoMoveValue for AggregatorValue {
    type Error = PartialVMError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        use AggregatorValue::*;
        use MoveTypeLayout::*;

        Ok(match (self, layout) {
            (Aggregator(v) | Snapshot(v), U64) => Value::u64(v as u64),
            (Aggregator(v) | Snapshot(v), U128) => Value::u128(v),
            (Derived(bytes), layout) if is_string_layout(layout) => bytes_to_string(bytes),
            (value, layout) => {
                return Err(
                    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
                        "Failed to convert {:?} into Move value with {} layout",
                        value, layout
                    )),
                )
            },
        })
    }
}

impl TryFromMoveValue for AggregatorValue {
    type Error = PartialVMError;
    // Need to distinguish between aggregators and snapshots of integer types.
    // TODO: We only need that because of the current enum-based implementations.
    type Hint = IdentifierMappingKind;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<Self, Self::Error> {
        use AggregatorValue::*;
        use IdentifierMappingKind as K;
        use MoveTypeLayout as L;

        Ok(match (hint, layout) {
            (K::Aggregator, L::U64) => Aggregator(value.value_as::<u64>()? as u128),
            (K::Aggregator, L::U128) => Aggregator(value.value_as::<u128>()?),
            (K::Snapshot, L::U64) => Snapshot(value.value_as::<u64>()? as u128),
            (K::Snapshot, L::U128) => Snapshot(value.value_as::<u128>()?),
            (K::Snapshot, layout) if is_string_layout(layout) => {
                let bytes = string_to_bytes(value.value_as::<Struct>()?)?;
                Derived(bytes)
            },
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
                        "Failed to convert Move value {:?} with {} layout into AggregatorValue",
                        value, layout
                    )),
                )
            },
        })
    }
}

// TODO see if we need both AggregatorValue and SnapshotValue. Also, maybe they should be nested
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
            AggregatorValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling SnapshotValue::try_from on AggregatorValue(Aggregator)",
            )),
            AggregatorValue::Snapshot(v) => Ok(SnapshotValue::Integer(v)),
            AggregatorValue::Derived(v) => Ok(SnapshotValue::String(v)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotToStringFormula {
    Concat { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl SnapshotToStringFormula {
    pub fn apply_to(&self, base: u128) -> Vec<u8> {
        match self {
            SnapshotToStringFormula::Concat { prefix, suffix } => {
                let middle_string = base.to_string();
                let middle = middle_string.as_bytes();
                let mut result = Vec::with_capacity(prefix.len() + middle.len() + suffix.len());
                result.extend(prefix);
                result.extend(middle);
                result.extend(suffix);
                result
            },
        }
    }
}
