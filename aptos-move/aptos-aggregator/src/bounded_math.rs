// Copyright © Aptos Foundation

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use aptos_logger::error;

use crate::delta_change_set::{EEXPECTED_OVERFLOW, EEXPECTED_UNDERFLOW, ECODE_INVARIANT_BROKEN};



#[derive(Debug)]
pub enum BoundedMathError {
    Overflow,
    Underflow,
}

/// Error for delta application. Can be used by delta partial functions
/// to return descriptive error messages and an appropriate error code.
pub(crate) fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

impl From<BoundedMathError> for PartialVMError {
    fn from(err: BoundedMathError) -> Self {
        match err {
            BoundedMathError::Overflow => abort_error(
                "Overflow",
                EEXPECTED_OVERFLOW,
            ),
            BoundedMathError::Underflow => abort_error(
                "Underflow",
                EEXPECTED_UNDERFLOW,
            ),
        }
    }
}

pub type BoundedMathResult<T> = ::std::result::Result<T, BoundedMathError>;

pub fn ok_overflow<T>(value: BoundedMathResult<T>) -> BoundedMathResult<Option<T>> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(BoundedMathError::Overflow) => Ok(None),
        Err(BoundedMathError::Underflow) => Err(BoundedMathError::Underflow),
    }
}

pub fn ok_underflow<T>(value: BoundedMathResult<T>) -> BoundedMathResult<Option<T>> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(BoundedMathError::Overflow) => Err(BoundedMathError::Overflow),
        Err(BoundedMathError::Underflow) => Ok(None),
    }
}

pub fn expect_ok<T>(value: BoundedMathResult<T>) -> PartialVMResult<T> {
    value.map_err(|e| {
        error!("Aggregator code invariant broken (there is a bug in the code)");
        abort_error(
            format!("Aggregator code invariant broken (there is a bug in the code), {:?}", e),
            ECODE_INVARIANT_BROKEN,
        )
    })
}

/// Implements application of `Addition` to `base`.
pub fn addition(base: u128, value: u128, limit: u128) -> BoundedMathResult<u128> {
    if limit < base || value > (limit - base) {
        Err(BoundedMathError::Overflow)
    } else {
        Ok(base + value)
    }
}

/// Implements application of `Subtraction` to `base`.
pub fn subtraction(base: u128, value: u128) -> BoundedMathResult<u128> {
    if value > base {
        Err(BoundedMathError::Underflow)
    } else {
        Ok(base - value)
    }
}

/// Describes the delta of an aggregator.
/// Rename to SignedU128 ?
#[derive(Clone, Copy, Hash, PartialOrd, Ord, Debug, PartialEq, Eq)]
pub enum DeltaValue {
    Positive(u128),
    Negative(u128),
}

impl DeltaValue {
    pub fn minus(&self) -> Self {
        match self {
            DeltaValue::Positive(value) => DeltaValue::Negative(*value),
            DeltaValue::Negative(value) => DeltaValue::Positive(*value),
        }
    }

    pub fn add(&self, other: &Self, max_value: u128) -> BoundedMathResult<Self> {
        // Another useful macro, this time for merging deltas with different signs, such
        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
        // and possibly flip a sign.
        macro_rules! update_different_sign {
            ($a:ident, $b:ident) => {
                if $a >= $b {
                    DeltaValue::Positive(subtraction(* $a, * $b)?)
                } else {
                    DeltaValue::Negative(subtraction(* $b, * $a)?)
                }
            };
        }

        Ok(match (self, other) {
            (DeltaValue::Positive(v1), DeltaValue::Positive(v2)) => DeltaValue::Positive(
                addition(*v1, *v2, max_value)?
            ),
            (DeltaValue::Positive(v1), DeltaValue::Negative(v2)) =>
                update_different_sign!(v1, v2),
            (DeltaValue::Negative(v1), DeltaValue::Positive(v2)) =>
                update_different_sign!(v2, v1),
            (DeltaValue::Negative(v1), DeltaValue::Negative(v2)) => DeltaValue::Negative(
                addition(*v1, *v2, max_value)?
            ),
        })
    }
}

/// Implements base + value
pub fn addition_deltavalue(
    base: u128,
    value: DeltaValue,
    max_value: u128,
) -> BoundedMathResult<u128> {
    match value {
        DeltaValue::Positive(value) => addition(base, value, max_value),
        DeltaValue::Negative(value) => subtraction(base, value),
    }
}
