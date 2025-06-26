// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::bounded_math::SignedU128;
use aptos_logger::error;
use aptos_types::delayed_fields::PanicError;
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::{
    delayed_values::{
        delayed_field_id::{DelayedFieldID, TryFromMoveValue},
        derived_string_snapshot::{
            bytes_and_width_to_derived_string_struct, derived_string_struct_to_bytes_and_length,
            is_derived_string_struct_layout,
        },
    },
    values::{Struct, Value},
};

// Wrapping another error, to add a variant that represents
// something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicOr<T: std::fmt::Debug> {
    CodeInvariantError(String),
    MissingNativeFunction(String),
    Or(T),
}

impl<T: std::fmt::Debug> PanicOr<T> {
    pub fn map_non_panic<E: std::fmt::Debug>(self, f: impl FnOnce(T) -> E) -> PanicOr<E> {
        match self {
            PanicOr::CodeInvariantError(msg) => PanicOr::CodeInvariantError(msg),
            PanicOr::MissingNativeFunction(msg) => PanicOr::MissingNativeFunction(msg),
            PanicOr::Or(value) => PanicOr::Or(f(value)),
        }
    }
}

pub fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed materialization code invariant broken (there is a bug in the code), {:?}",
        message
    );
    error!("{}", msg);
    PanicError::CodeInvariantError(msg)
}

pub fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> Result<V, PanicError> {
    value.map_err(|e| code_invariant_error(format!("Expected Ok, got Err({:?})", e)))
}

impl<T: std::fmt::Debug> From<PanicError> for PanicOr<T> {
    fn from(err: PanicError) -> Self {
        match err {
            PanicError::CodeInvariantError(e) => PanicOr::CodeInvariantError(e),
            PanicError::MissingNativeFunction(e) => PanicOr::MissingNativeFunction(e),
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
            PanicOr::CodeInvariantError(_) => {
                StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR
            },
            PanicOr::MissingNativeFunction(_) => StatusCode::MISSING_NATIVE_FUNCTION,
            PanicOr::Or(_) => StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
        }
    }
}

impl<T: std::fmt::Debug> From<PanicOr<T>> for PartialVMError {
    fn from(err: PanicOr<T>) -> Self {
        match err {
            PanicOr::CodeInvariantError(msg) => {
                PartialVMError::new(StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR)
                    .with_message(msg)
            },
            PanicOr::MissingNativeFunction(msg) => {
                PartialVMError::new(StatusCode::MISSING_NATIVE_FUNCTION).with_message(msg)
            },
            PanicOr::Or(err) => PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                .with_message(format!("{:?}", err)),
        }
    }
}

/// Different reasons for why applying new start_value doesn't
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
    /// Merging two Deltas (value only) failed.
    DeltaMerge {
        base_delta: SignedU128,
        delta: SignedU128,
        max_value: u128,
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
    InconsistentRead,
}

impl NonPanic for DelayedFieldsSpeculativeError {}

/// Value of a DelayedField (i.e. aggregator or snapshot)
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelayedFieldValue {
    Aggregator(u128),
    Snapshot(u128),
    // TODO[agg_v2](optimize) probably change to Derived(Arc<Vec<u8>>) to make copying predictably costly
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

    pub fn try_into_move_value(
        self,
        layout: &MoveTypeLayout,
        width: u32,
    ) -> Result<Value, PartialVMError> {
        use DelayedFieldValue::*;
        use MoveTypeLayout::*;

        Ok(match (self, layout) {
            (Aggregator(v) | Snapshot(v), U64) => {
                if width != 8 {
                    return Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                        .with_message(format!("Expected width 8 for U64, got {}", width)));
                }
                Value::u64(v as u64)
            },
            (Aggregator(v) | Snapshot(v), U128) => {
                if width != 16 {
                    return Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                        .with_message(format!("Expected width 16 for U128, got {}", width)));
                }
                Value::u128(v)
            },
            (Derived(bytes), layout) if is_derived_string_struct_layout(layout) => {
                bytes_and_width_to_derived_string_struct(bytes, width as usize)?
            },
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

    /// Approximate memory consumption of current DelayedFieldValue
    pub fn get_approximate_memory_size(&self) -> usize {
        // 32 + len
        std::mem::size_of::<DelayedFieldValue>()
            + match &self {
                DelayedFieldValue::Aggregator(_) | DelayedFieldValue::Snapshot(_) => 0,
                // additional allocated memory for the data:
                DelayedFieldValue::Derived(v) => v.len(),
            }
    }
}

impl TryFromMoveValue for DelayedFieldValue {
    type Error = PartialVMError;
    // Need to distinguish between aggregators and snapshots of integer types.
    // TODO[agg_v2](cleanup): We only need that because of the current enum-based
    // implementations. See if we want to keep that separation, or clean it up.
    type Hint = IdentifierMappingKind;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<(Self, u32), Self::Error> {
        use DelayedFieldValue::*;
        use IdentifierMappingKind as K;
        use MoveTypeLayout as L;

        Ok(match (hint, layout) {
            (K::Aggregator, L::U64) => (Aggregator(value.value_as::<u64>()? as u128), 8),
            (K::Aggregator, L::U128) => (Aggregator(value.value_as::<u128>()?), 16),
            (K::Snapshot, L::U64) => (Snapshot(value.value_as::<u64>()? as u128), 8),
            (K::Snapshot, L::U128) => (Snapshot(value.value_as::<u128>()?), 16),
            (K::DerivedString, layout) if is_derived_string_struct_layout(layout) => {
                let (bytes, width) =
                    derived_string_struct_to_bytes_and_length(value.value_as::<Struct>()?)?;
                (Derived(bytes), width)
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

pub enum ReadPosition {
    BeforeCurrentTxn,
    AfterCurrentTxn,
}
