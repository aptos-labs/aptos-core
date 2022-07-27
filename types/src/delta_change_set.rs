// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op. Every delta is is a state key
//! (for accessing the storage) and an operation: a partial function with a
//! postcondition.

use crate::state_store::state_key::StateKey;

/// Specifies different delta partial function specifications.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaOp {
    /// Addition of `value` which overflows on `limit`.
    Addition { value: u128, limit: u128 },
    /// Subtraction of `value` which cannot go below zero.
    Subtraction { value: u128 },
}

impl DeltaOp {
    /// Returns optional result of delta application to `base` (None if
    /// postocndition not satisfied).
    pub fn apply_to(&self, base: u128) -> Option<u128> {
        match self {
            DeltaOp::Addition { value, limit } => addition(base, *value, *limit),
            DeltaOp::Subtraction { value } => subtraction(base, *value),
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

impl std::fmt::Debug for DeltaOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeltaOp::Addition { value, limit } => {
                write!(f, "+{} ensures result <= {}", value, limit)
            }
            DeltaOp::Subtraction { value } => {
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
pub fn deserialize(value_bytes: &[u8]) -> u128 {
    bcs::from_bytes(value_bytes).expect("unexpected deserialization error")
}

/// `DeltaChangeSet` contains all access paths that one transaction wants to update with deltas.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeltaChangeSet {
    delta_change_set: Vec<(StateKey, DeltaOp)>,
}

impl DeltaChangeSet {
    pub fn empty() -> Self {
        DeltaChangeSet {
            delta_change_set: vec![],
        }
    }

    pub fn new(delta_change_set: Vec<(StateKey, DeltaOp)>) -> Self {
        DeltaChangeSet { delta_change_set }
    }

    pub fn push(&mut self, delta: (StateKey, DeltaOp)) {
        self.delta_change_set.push(delta);
    }

    pub fn pop(&mut self) {
        self.delta_change_set.pop();
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.delta_change_set.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::{assert_matches, assert_some_eq};

    fn addition(value: u128, limit: u128) -> DeltaOp {
        DeltaOp::Addition { value, limit }
    }

    fn subtraction(value: u128) -> DeltaOp {
        DeltaOp::Subtraction { value }
    }

    #[test]
    fn test_delta_addition() {
        let add5 = addition(5, 100);
        assert_some_eq!(add5.apply_to(0), 5);
        assert_some_eq!(add5.apply_to(5), 10);
        assert_some_eq!(add5.apply_to(95), 100);

        assert_matches!(add5.apply_to(96), None);
    }

    #[test]
    fn test_delta_subtraction() {
        let sub5 = subtraction(5);
        assert_matches!(sub5.apply_to(0), None);
        assert_matches!(sub5.apply_to(1), None);

        assert_some_eq!(sub5.apply_to(5), 0);
        assert_some_eq!(sub5.apply_to(100), 95);
    }
}
