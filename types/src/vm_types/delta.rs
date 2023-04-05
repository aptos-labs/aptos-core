// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Formatter, Result};
use move_core_types::vm_status::StatusCode;
use move_vm_types::natives::function::{PartialVMError, PartialVMResult};

/// Error code for overflows.
const LIMIT_OVERFLOW: u64 = 0x02_0001;
/// Error code for going below zero.
const BELOW_ZERO: u64 = 0x02_0002;

/// Different delta functions.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaUpdate {
    Plus(u128),
    Minus(u128),
}

/// Represents a partial update to integers in the global state.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct DeltaOp {
    /// Maximum positive delta seen during execution.
    max_positive: u128,
    /// Smallest negative delta seen during execution.
    min_negative: u128,
    /// Post-condition: delta overflows on exceeding this limit or going below
    /// zero.
    limit: u128,
    /// Delta which is the result of the execution.
    update: DeltaUpdate,
}

/// Adds `value` to `base`. Returns error if the result of addition is greater than `limit`.
pub fn addition(base: u128, value: u128, limit: u128) -> PartialVMResult<u128> {
    if limit < base || value > (limit - base) {
        Err(abort_error(
            format!("overflow when adding {} to {}", value, base),
            LIMIT_OVERFLOW,
        ))
    } else {
        Ok(base + value)
    }
}

/// Subtracts `value` from `base`. Returns error if the result of subtraction is below zero.
pub fn subtraction(base: u128, value: u128) -> PartialVMResult<u128> {
    if value > base {
        Err(abort_error(
            format!("underflow when subtracting {} from {}", value, base),
            BELOW_ZERO,
        ))
    } else {
        Ok(base - value)
    }
}

impl DeltaOp {
    pub fn new(update: DeltaUpdate, limit: u128, max_positive: u128, min_negative: u128) -> Self {
        Self {
            max_positive,
            min_negative,
            limit,
            update,
        }
    }

    /// Returns the kind of update for this delta op.
    pub fn update(&self) -> DeltaUpdate {
        self.update
    }

    /// Returns the result of delta application to `base` or an error if
    /// the application fails.
    pub fn apply_to(&self, base: u128) -> PartialVMResult<u128> {
        // First, validate if delta op can be applied to `base`. Note that
        // this is possible if the values observed during execution didn't
        // overflow or dropped below zero. The check can be emulated by actually
        // doing addition and subtraction.
        addition(base, self.max_positive, self.limit)?;
        subtraction(base, self.min_negative)?;

        // If delta has been successfully validated, apply the update.
        match self.update {
            DeltaUpdate::Plus(value) => addition(base, value, self.limit),
            DeltaUpdate::Minus(value) => subtraction(base, value),
        }
    }
}

impl Debug for DeltaOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.update {
            DeltaUpdate::Plus(value) => {
                write!(
                    f,
                    "+{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
            },
            DeltaUpdate::Minus(value) => {
                write!(
                    f,
                    "-{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
            },
        }
    }
}

/// Returns partial VM error on abort. Can be used to return descriptive error messages and
/// an appropriate error code.
fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}
