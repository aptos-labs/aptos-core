// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op. Every delta is is a state key
//! (for accessing the storage) and an operation: a partial function with a
//! postcondition.

use crate::module::AGGREGATOR_MODULE;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    vm_status::{StatusCode, VMStatus},
    write_set::{WriteOp, WriteSetMut},
};
use move_deps::move_binary_format::errors::{Location, PartialVMError, PartialVMResult};

/// When `Addition` operation overflows the `limit`.
const EADD_OVERFLOW: u64 = 0x02_0001;

/// When `Subtraction` operation goes below zero.
const ESUB_UNDERFLOW: u64 = 0x02_0002;

/// Specifies different delta partial function specifications.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaOp {
    /// Addition of `value` which overflows on `limit`.
    Addition { value: u128, limit: u128 },
    /// Subtraction of `value` which cannot go below zero.
    Subtraction { value: u128 },
}

impl DeltaOp {
    /// Returns the result of delta application to `base` or error if
    /// postcondition is not satisfied.
    pub fn apply_to(&self, base: u128) -> PartialVMResult<u128> {
        match self {
            DeltaOp::Addition { value, limit } => addition(base, *value, *limit),
            DeltaOp::Subtraction { value } => subtraction(base, *value),
        }
    }

    /// Consumes a single delta and tries to materialize it with a given state
    /// key. If materialization succeeds, a write op is produced. Otherwise, an
    /// error VM status is returned.
    pub fn try_into_write_op(
        self,
        state_view: &impl StateView,
        state_key: &StateKey,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        state_view
            .get_state_value(state_key)
            .map_err(|_| VMStatus::Error(StatusCode::STORAGE_ERROR))
            .and_then(|maybe_bytes| {
                match maybe_bytes {
                    Some(bytes) => {
                        let base = deserialize(&bytes);
                        self.apply_to(base)
                            .map_err(|partial_error| {
                                // If delta application fails, transform partial VM
                                // error into an appropriate VM status.
                                partial_error
                                    .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                                    .into_vm_status()
                            })
                            .map(|result| WriteOp::Modification(serialize(&result)))
                    }
                    // Something is wrong, the value to which we apply delta should
                    // always exist. Guard anyway.
                    None => Err(VMStatus::Error(StatusCode::STORAGE_ERROR)),
                }
            })
    }
}

/// Implements application of `Addition` to `base`.
pub fn addition(base: u128, value: u128, limit: u128) -> PartialVMResult<u128> {
    if limit < base || value > (limit - base) {
        Err(abort_error(
            format!("overflow when adding {} to {}", value, base),
            EADD_OVERFLOW,
        ))
    } else {
        Ok(base + value)
    }
}

/// Implements application of `Subtraction` to `base`.
pub fn subtraction(base: u128, value: u128) -> PartialVMResult<u128> {
    if value > base {
        Err(abort_error(
            format!("underflow when subtracting {} from {}", value, base),
            ESUB_UNDERFLOW,
        ))
    } else {
        Ok(base - value)
    }
}

/// Returns partial VM error on abort. Can be used by delta partial functions
/// to return descriptive error messages and an appropriate error code.
fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
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
    bcs::to_bytes(value).expect("unexpected serialization error in aggregator")
}

/// Deserializes value for delta application.
pub fn deserialize(value_bytes: &[u8]) -> u128 {
    bcs::from_bytes(value_bytes).expect("unexpected deserialization error in aggregator")
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

    /// Consumes the delta change set and tries to materialize it. Returns a
    /// mutable write set if materialization succeeds (mutability since we want
    /// to merge these writes with transaction outputs).
    pub fn try_into_write_set_mut(
        self,
        state_view: &impl StateView,
    ) -> anyhow::Result<WriteSetMut, VMStatus> {
        let mut materialized_write_set = vec![];
        for (state_key, delta_op) in self.delta_change_set {
            let write_op = delta_op.try_into_write_op(state_view, &state_key)?;
            materialized_write_set.push((state_key, write_op));
        }

        // All deltas are applied successfully.
        Ok(WriteSetMut::new(materialized_write_set))
    }
}

impl ::std::iter::IntoIterator for DeltaChangeSet {
    type Item = (StateKey, DeltaOp);
    type IntoIter = ::std::vec::IntoIter<(StateKey, DeltaOp)>;

    fn into_iter(self) -> Self::IntoIter {
        self.delta_change_set.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_state_view::state_storage_usage::StateStorageUsage;
    use claim::{assert_err, assert_matches, assert_ok_eq};
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    fn addition(value: u128, limit: u128) -> DeltaOp {
        DeltaOp::Addition { value, limit }
    }

    fn subtraction(value: u128) -> DeltaOp {
        DeltaOp::Subtraction { value }
    }

    #[test]
    fn test_delta_addition() {
        let add5 = addition(5, 100);
        assert_ok_eq!(add5.apply_to(0), 5);
        assert_ok_eq!(add5.apply_to(5), 10);
        assert_ok_eq!(add5.apply_to(95), 100);

        assert_err!(add5.apply_to(96));
    }

    #[test]
    fn test_delta_subtraction() {
        let sub5 = subtraction(5);
        assert_err!(sub5.apply_to(0));
        assert_err!(sub5.apply_to(1));

        assert_ok_eq!(sub5.apply_to(5), 0);
        assert_ok_eq!(sub5.apply_to(100), 95);
    }

    #[derive(Default)]
    pub struct FakeView {
        data: HashMap<StateKey, Vec<u8>>,
    }

    impl StateView for FakeView {
        fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(self.data.get(state_key).cloned())
        }

        fn is_genesis(&self) -> bool {
            self.data.is_empty()
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            Ok(StateStorageUsage::new_untracked())
        }
    }

    static KEY: Lazy<StateKey> = Lazy::new(|| StateKey::Raw(String::from("test-key").into_bytes()));

    #[test]
    fn test_failed_delta_application() {
        let state_view = FakeView::default();
        let delta_op = addition(10, 1000);
        assert_matches!(
            delta_op.try_into_write_op(&state_view, &*KEY),
            Err(VMStatus::Error(StatusCode::STORAGE_ERROR))
        );
    }

    #[test]
    fn test_successful_delta_application() {
        let mut state_view = FakeView::default();
        state_view.data.insert(KEY.clone(), serialize(&100));

        // Both addition and subtraction should succeed!
        let add_op = addition(100, 200);
        let sub_op = subtraction(100);

        let add_result = add_op.try_into_write_op(&state_view, &*KEY);
        assert_ok_eq!(add_result, WriteOp::Modification(serialize(&200)));

        let sub_result = sub_op.try_into_write_op(&state_view, &*KEY);
        assert_ok_eq!(sub_result, WriteOp::Modification(serialize(&0)));
    }

    #[test]
    fn test_unsuccessful_delta_application() {
        let mut state_view = FakeView::default();
        state_view.data.insert(KEY.clone(), serialize(&100));

        // Both addition and subtraction should fail!
        let add_op = addition(15, 100);
        let sub_op = subtraction(101);

        assert_matches!(
            add_op.try_into_write_op(&state_view, &*KEY),
            Err(VMStatus::MoveAbort(_, EADD_OVERFLOW))
        );
        assert_matches!(
            sub_op.try_into_write_op(&state_view, &*KEY),
            Err(VMStatus::MoveAbort(_, ESUB_UNDERFLOW))
        );
    }
}
