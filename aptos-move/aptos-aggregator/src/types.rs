// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::SignedU128,
    utils::{bytes_to_string, is_string_layout, string_to_bytes},
};
use aptos_logger::error;
// TODO: After aggregators_v2 branch land, consolidate these, instead of using alias here
pub use aptos_types::aggregator::{DelayedFieldID, PanicError, TryFromMoveValue, TryIntoMoveValue};
use aptos_types::state_store::{state_key::StateKey, table::TableHandle};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress,
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::values::{Struct, Value};

// Wrapping another error, to add a variant that represents
// something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicOr<T: std::fmt::Debug> {
    CodeInvariantError(String),
    Or(T),
}

pub fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    error!("{}", msg);
    PanicError::CodeInvariantError(msg)
}

pub fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> Result<V, PanicError> {
    value.map_err(code_invariant_error)
}

impl<T: std::fmt::Debug> From<PanicError> for PanicOr<T> {
    fn from(err: PanicError) -> Self {
        match err {
            PanicError::CodeInvariantError(e) => PanicOr::CodeInvariantError(e),
        }
    }
}

pub trait NonPanic {}

impl<T: std::fmt::Debug + NonPanic> From<T> for PanicOr<T> {
    fn from(err: T) -> Self {
        PanicOr::Or(err)
    }
}

impl From<DelayedFieldsSpeculativeError> for PartialVMError {
    fn from(err: DelayedFieldsSpeculativeError) -> Self {
        PartialVMError::from(PanicOr::from(err))
    }
}

impl<T: std::fmt::Debug> From<&PanicOr<T>> for StatusCode {
    fn from(err: &PanicOr<T>) -> Self {
        match err {
            PanicOr::CodeInvariantError(_) => StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
            PanicOr::Or(_) => StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
        }
    }
}

impl<T: std::fmt::Debug> From<PanicOr<T>> for PartialVMError {
    fn from(err: PanicOr<T>) -> Self {
        match err {
            PanicOr::CodeInvariantError(msg) => {
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                    .with_message(msg)
            },
            PanicOr::Or(err) => PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                .with_message(format!("{:?}", err)),
        }
    }
}

/// Different reaons for why applying new start_value doesn't
/// satisfy history bounds
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaApplicationFailureReason {
    /// max_achieved wouldn't be within bounds
    Overflow,
    /// min_achieved wouldn't be within bounds
    Underflow,
    /// min_overflow wouldn't cause overflow any more
    ExpectedOverflow,
    /// max_underflow wouldn't cause underflow any more
    ExpectedUnderflow,
}

/// Different reasons for why merging two Deltas (value + history) failed,
/// because newer one couldn't be offsetted by the delta value
/// of the older one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaHistoryMergeOffsetFailureReason {
    /// If we offset achieved, it exceeds bounds
    AchievedExceedsBounds,
    /// if we offset failure (overflow/underflow), it cannot
    /// exceed bounds any more (because it went on the opposite side of 0)
    FailureNotExceedingBoundsAnyMore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelayedFieldsSpeculativeError {
    /// DelayedField with given ID couldn't be found
    /// (due to speculative nature), but must exist.
    NotFound(DelayedFieldID),
    /// Applying new start_value doesn't satisfy history bounds.
    DeltaApplication {
        base_value: u128,
        max_value: u128,
        delta: SignedU128,
        reason: DeltaApplicationFailureReason,
    },
    /// Merging two Deltas (value + history) failed, because newer
    /// one couldn't be offsetted by the delta value of the older one.
    DeltaHistoryMergeOffset {
        target: u128,
        delta: SignedU128,
        max_value: u128,
        reason: DeltaHistoryMergeOffsetFailureReason,
    },
    /// Merging two Deltas (value + history) failed, because no value
    /// could satisfy both achieved and failure (overflow/underflow)
    /// bounds, as they now overlap.
    DeltaHistoryMergeAchievedAndFailureOverlap {
        achieved: SignedU128,
        overflow: SignedU128,
    },
}

impl NonPanic for DelayedFieldsSpeculativeError {}

/// Uniquely identifies aggregator or aggregator snapshot instances in
/// extension and possibly storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregatorVersionedID {
    // Aggregator V1 is implemented as a state item, and so can be queried by
    // the state key.
    V1(StateKey),
    // Aggregator V2 is embedded into resources, and uses ephemeral identifiers
    // which are unique per block.
    V2(DelayedFieldID),
}

impl AggregatorVersionedID {
    pub fn v1(handle: TableHandle, key: AccountAddress) -> Self {
        let state_key = StateKey::table_item(handle, key.to_vec());
        Self::V1(state_key)
    }

    pub fn v2(value: u64) -> Self {
        Self::V2(DelayedFieldID::new(value))
    }
}

impl TryFrom<AggregatorVersionedID> for StateKey {
    type Error = PanicError;

    fn try_from(vid: AggregatorVersionedID) -> Result<Self, Self::Error> {
        match vid {
            AggregatorVersionedID::V1(state_key) => Ok(state_key),
            AggregatorVersionedID::V2(_) => Err(code_invariant_error("wrong version id")),
        }
    }
}

/// Value of a DelayedField (i.e. aggregator or snapshot)
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelayedFieldValue {
    Aggregator(u128),
    Snapshot(u128),
    // TODO probably change to Derived(Arc<Vec<u8>>) to make copying predictably costly
    Derived(Vec<u8>),
}

impl DelayedFieldValue {
    pub fn into_aggregator_value(self) -> Result<u128, PanicError> {
        match self {
            DelayedFieldValue::Aggregator(value) => Ok(value),
            DelayedFieldValue::Snapshot(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on Snapshot value",
            )),
            DelayedFieldValue::Derived(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on String SnapshotValue",
            )),
        }
    }

    pub fn into_snapshot_value(self) -> Result<u128, PanicError> {
        match self {
            DelayedFieldValue::Snapshot(value) => Ok(value),
            DelayedFieldValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling into_snapshot_value on Aggregator value",
            )),
            DelayedFieldValue::Derived(_) => Err(code_invariant_error(
                "Tried calling into_snapshot_value on String SnapshotValue",
            )),
        }
    }

    pub fn into_derived_value(self) -> Result<Vec<u8>, PanicError> {
        match self {
            DelayedFieldValue::Derived(value) => Ok(value),
            DelayedFieldValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling into_derived_value on Aggregator value",
            )),
            DelayedFieldValue::Snapshot(_) => Err(code_invariant_error(
                "Tried calling into_derived_value on Snapshot value",
            )),
        }
    }
}

impl TryIntoMoveValue for DelayedFieldValue {
    type Error = PartialVMError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        use DelayedFieldValue::*;
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

impl TryFromMoveValue for DelayedFieldValue {
    type Error = PartialVMError;
    // Need to distinguish between aggregators and snapshots of integer types.
    // TODO: We only need that because of the current enum-based implementations.
    type Hint = IdentifierMappingKind;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<Self, Self::Error> {
        use DelayedFieldValue::*;
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
    pub fn into_aggregator_value(self) -> Result<u128, PanicError> {
        match self {
            SnapshotValue::Integer(value) => Ok(value),
            SnapshotValue::String(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on String SnapshotValue",
            )),
        }
    }
}

impl TryFrom<DelayedFieldValue> for SnapshotValue {
    type Error = PanicError;

    fn try_from(value: DelayedFieldValue) -> Result<SnapshotValue, PanicError> {
        match value {
            DelayedFieldValue::Aggregator(_) => Err(code_invariant_error(
                "Tried calling SnapshotValue::try_from on AggregatorValue(Aggregator)",
            )),
            DelayedFieldValue::Snapshot(v) => Ok(SnapshotValue::Integer(v)),
            DelayedFieldValue::Derived(v) => Ok(SnapshotValue::String(v)),
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

pub enum ReadPosition {
    BeforeCurrentTxn,
    AfterCurrentTxn,
}
