// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op (note: this is a temporary
//! solution and ideally we should modify `ChangeSet` and `TransactionOutput`
//! to keep deltas internal to executor). Every delta is parametrized by an
//! operation: a partial function with a postcondition.

/// Specifies different delta partial function specifications.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaOperation {
    // Addition of `value` which overflows on `limit`.
    Addition { value: u128, limit: u128 },
    // Subtraction of `value` which cannot go below zero.
    Subtraction { value: u128 },
}

impl DeltaOperation {
    /// Returns optional result of delta application to `base` (None if
    /// postocndition not satisfied).
    pub fn apply_to(&self, base: u128) -> Option<u128> {
        match self {
            DeltaOperation::Addition { value, limit } => addition(base, *value, *limit),
            DeltaOperation::Subtraction { value } => subtraction(base, *value),
        }
    }
}

/// Implements application of `Addition` to `base`.
fn addition(base: u128, value: u128, limit: u128) -> Option<u128> {
    if value > (limit - base) {
        None
    } else {
        Some(base + value)
    }
}

/// Implements application of `Subtraction` to `base`.
fn subtraction(base: u128, value: u128) -> Option<u128> {
    if value > base {
        None
    } else {
        Some(base - value)
    }
}

impl std::fmt::Debug for DeltaOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeltaOperation::Addition { value, limit } => {
                write!(f, "+{} ensures result <= {}", value, limit)
            }
            DeltaOperation::Subtraction { value } => {
                write!(f, "-{} ensures 0 <= result", value)
            }
        }
    }
}

/// Serializes value after delta application.
pub fn serialize(value: &u128) -> Vec<u8> {
    bcs::to_bytes(value).expect("unexpected serialization error")
}

/// Deserializes value for delta application.
pub fn deserialize(value_bytes: &Vec<u8>) -> u128 {
    bcs::from_bytes(value_bytes).expect("unexpected deserialization error")
}
