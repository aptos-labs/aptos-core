// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::SignedU128,
    utils::{
        bytes_to_string, from_utf8_bytes, is_string_layout, string_to_bytes, to_utf8_bytes,
        u128_to_u64,
    },
};
use aptos_logger::error;
use aptos_types::state_store::{state_key::StateKey, table::TableHandle};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress,
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::values::{Struct, Value};

/// Types which implement this trait can be converted to a Move value.
pub trait TryIntoMoveValue: Sized {
    type Error: std::fmt::Debug;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error>;
}

/// Types which implement this trait can be constructed from a Move value.
pub trait TryFromMoveValue: Sized {
    // Allows to pass extra information from the caller.
    type Hint;
    type Error: std::fmt::Debug;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<Self, Self::Error>;
}

// represents something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanicError(String);

impl ToString for PanicError {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

// Wrapping another error, to add a variant that represents
// something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicOr<T: std::fmt::Debug> {
    CodeInvariantError(String),
    Or(T),
}

pub type PanicOrResult<T, E> = Result<T, PanicOr<E>>;

pub fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    error!("{}", msg);
    PanicError(msg)
}

pub fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> Result<V, PanicError> {
    value.map_err(code_invariant_error)
}

impl<T: std::fmt::Debug> From<PanicError> for PanicOr<T> {
    fn from(err: PanicError) -> Self {
        PanicOr::CodeInvariantError(err.0)
    }
}

pub trait NonPanic {}
// impl NonPanic for f64 {}

impl<T: std::fmt::Debug + NonPanic> From<T> for PanicOr<T> {
    fn from(err: T) -> Self {
        PanicOr::Or(err)
    }
}

impl From<PanicError> for PartialVMError {
    fn from(err: PanicError) -> Self {
        PartialVMError::from(PanicOr::<()>::from(err))
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
            PanicOr::Or(_) => StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR,
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
            PanicOr::Or(err) => {
                PartialVMError::new(StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR)
                    .with_message(format!("{:?}", err))
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaApplicationFailureReason {
    Overflow,
    Underflow,
    ExpectedOverflow,
    ExpectedUnderflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaHistoryMergeOffsetFailureReason {
    AchievedExceedsBounds,
    FailureNotExceedingBoundsAnyMore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelayedFieldsSpeculativeError {
    NotFound(AggregatorID),
    DeltaApplication {
        base_value: u128,
        max_value: u128,
        delta: SignedU128,
        reason: DeltaApplicationFailureReason,
    },
    DeltaHistoryMergeOffset {
        target: u128,
        delta: SignedU128,
        max_value: u128,
        reason: DeltaHistoryMergeOffsetFailureReason,
    },
    DeltaHistoryMergeAchievedAndOverflowOverlap {
        achieved: SignedU128,
        overflow: SignedU128,
    },
}

impl NonPanic for DelayedFieldsSpeculativeError {}

// TODO To be renamed to DelayedFieldID
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
    type Error = PanicError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.0),
            MoveTypeLayout::U128 => Value::u128(self.0 as u128),
            layout if is_string_layout(layout) => bytes_to_string(to_utf8_bytes(self.0)),
            _ => {
                return Err(code_invariant_error(format!(
                    "Failed to convert {:?} into a Move value with {} layout",
                    self, layout
                )))
            },
        })
    }
}

impl TryFromMoveValue for AggregatorID {
    type Error = PanicError;
    type Hint = ();

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        _hint: &Self::Hint,
    ) -> Result<Self, Self::Error> {
        // Since we put the value there, we should be able to read it back,
        // unless there is a bug in the code - so we expect_ok() throughout.
        expect_ok(match layout {
            MoveTypeLayout::U64 => value.value_as::<u64>(),
            MoveTypeLayout::U128 => value.value_as::<u128>().and_then(u128_to_u64),
            layout if is_string_layout(layout) => value
                .value_as::<Struct>()
                .and_then(string_to_bytes)
                .and_then(from_utf8_bytes),
            // We use value to ID conversion in serialization.
            _ => Err(
                PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
                    "Failed to convert a Move value with {} layout into an identifier",
                    layout
                )),
            ),
        })
        .map(Self::new)
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
    type Error = PanicError;

    fn try_from(vid: AggregatorVersionedID) -> Result<Self, Self::Error> {
        match vid {
            AggregatorVersionedID::V1(state_key) => Ok(state_key),
            AggregatorVersionedID::V2(_) => Err(code_invariant_error("wrong version id")),
        }
    }
}

// TODO To be renamed to DelayedFieldValue
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorValue {
    Aggregator(u128),
    Snapshot(u128),
    // TODO probably change to Derived(Arc<Vec<u8>>) to make copying predictably costly
    Derived(Vec<u8>),
}

impl AggregatorValue {
    pub fn into_aggregator_value(self) -> Result<u128, PanicError> {
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

    pub fn into_snapshot_value(self) -> Result<u128, PanicError> {
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

    pub fn into_derived_value(self) -> Result<Vec<u8>, PanicError> {
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
    pub fn into_aggregator_value(self) -> Result<u128, PanicError> {
        match self {
            SnapshotValue::Integer(value) => Ok(value),
            SnapshotValue::String(_) => Err(code_invariant_error(
                "Tried calling into_aggregator_value on String SnapshotValue",
            )),
        }
    }
}

impl TryFrom<AggregatorValue> for SnapshotValue {
    type Error = PanicError;

    fn try_from(value: AggregatorValue) -> Result<SnapshotValue, PanicError> {
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

pub enum ReadPosition {
    BeforeCurrentTxn,
    AfterCurrentTxn,
}
