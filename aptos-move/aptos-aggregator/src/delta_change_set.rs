// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op. Every delta is is a state key
//! (for accessing the storage) and an operation: a partial function with a
//! postcondition.

use std::collections::BTreeMap;

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

/// Represents an update from aggregator's operation.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct DeltaOp {
    /// Maximum positive delta seen during execution.
    max_positive: u128,
    /// Smallest negative delta seen during execution.
    min_negative: u128,
    /// Postcondition: delta overflows on exceeding this limit or going below
    /// zero.
    limit: u128,
    /// Delta whoch is the result of the execution.
    update: DeltaUpdate,
}

/// Different delta functions.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaUpdate {
    Plus(u128),
    Minus(u128),
}

impl DeltaOp {
    /// Creates a new delta op.
    pub fn new(update: DeltaUpdate, limit: u128, max_positive: u128, min_negative: u128) -> Self {
        Self {
            max_positive,
            min_negative,
            limit,
            update,
        }
    }

    /// Returns the kind of update for the delta op.
    pub fn get_update(&self) -> DeltaUpdate {
        self.update
    }

    /// Returns the result of delta application to `base` or error if
    /// postcondition is not satisfied.
    pub fn apply_to(&self, base: u128) -> PartialVMResult<u128> {
        // First, validate if delta op can be applied to `base`. Note that
        // this is possible if the values observed during execution didn't
        // overflow or dropped below zero. The check can be emulated by actually
        // doing addition and subtraction.
        addition(base, self.max_positive, self.limit)?;
        subtraction(base, self.min_negative)?;

        // If delta has been sucessfully validated, apply the update.
        match self.update {
            DeltaUpdate::Plus(value) => addition(base, value, self.limit),
            DeltaUpdate::Minus(value) => subtraction(base, value),
        }
    }

    /// Aggregates another delta into `self`.
    pub fn merge_with(&mut self, other: DeltaOp) -> PartialVMResult<()> {
        use DeltaUpdate::*;

        Ok(match (self.update, other.update) {
            (Plus(value), Plus(other_value)) => {
                let new_value = addition(value, other_value, self.limit)?;
                self.update = Plus(new_value);
            }
            (Plus(value), Minus(other_value)) | (Minus(other_value), Plus(value)) => {
                if value >= other_value {
                    let new_value = subtraction(value, other_value)?;
                    self.update = Plus(new_value);
                } else {
                    let new_value = subtraction(other_value, value)?;
                    self.update = Minus(new_value);
                }
            }
            (Minus(value), Minus(other_value)) => {
                let new_value = addition(value, other_value, self.limit)?;
                self.update = Minus(new_value);
            }
        })
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
        match self.update {
            DeltaUpdate::Plus(value) => {
                write!(
                    f,
                    "+{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
            }
            DeltaUpdate::Minus(value) => {
                write!(
                    f,
                    "-{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
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

// Helper for tests, #[cfg(test)] doesn't work for cross-crate.
pub fn delta_sub(v: u128, limit: u128) -> DeltaOp {
    DeltaOp::new(DeltaUpdate::Minus(v), limit, 0, v)
}

// Helper for tests, #[cfg(test)] doesn't work for cross-crate.
pub fn delta_add(v: u128, limit: u128) -> DeltaOp {
    DeltaOp::new(DeltaUpdate::Plus(v), limit, v, 0)
}

/// `DeltaChangeSet` contains all access paths that one transaction wants to update with deltas.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeltaChangeSet {
    delta_change_set: BTreeMap<StateKey, DeltaOp>,
}

impl DeltaChangeSet {
    pub fn empty() -> Self {
        DeltaChangeSet {
            delta_change_set: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.delta_change_set.len()
    }

    pub fn new(delta_change_set: impl IntoIterator<Item = (StateKey, DeltaOp)>) -> Self {
        DeltaChangeSet {
            delta_change_set: delta_change_set.into_iter().collect(),
        }
    }

    pub fn insert(&mut self, delta: (StateKey, DeltaOp)) {
        self.delta_change_set.insert(delta.0, delta.1);
    }

    pub fn remove(&mut self, key: &StateKey) -> Option<DeltaOp> {
        self.delta_change_set.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> ::std::collections::btree_map::Iter<'_, StateKey, DeltaOp> {
        self.into_iter()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.delta_change_set.is_empty()
    }

    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<StateKey, DeltaOp> {
        &mut self.delta_change_set
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

impl<'a> IntoIterator for &'a DeltaChangeSet {
    type Item = (&'a StateKey, &'a DeltaOp);
    type IntoIter = ::std::collections::btree_map::Iter<'a, StateKey, DeltaOp>;

    fn into_iter(self) -> Self::IntoIter {
        self.delta_change_set.iter()
    }
}

impl ::std::iter::IntoIterator for DeltaChangeSet {
    type Item = (StateKey, DeltaOp);
    type IntoIter = ::std::collections::btree_map::IntoIter<StateKey, DeltaOp>;

    fn into_iter(self) -> Self::IntoIter {
        self.delta_change_set.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::state_storage_usage::StateStorageUsage;
    use claims::{assert_err, assert_matches, assert_ok, assert_ok_eq};
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    #[test]
    fn test_delta_addition() {
        let add5 = delta_add(5, 100);
        assert_ok_eq!(add5.apply_to(0), 5);
        assert_ok_eq!(add5.apply_to(5), 10);
        assert_ok_eq!(add5.apply_to(95), 100);

        assert_err!(add5.apply_to(96));
    }

    #[test]
    fn test_delta_subtraction() {
        let sub5 = delta_sub(5, 100);
        assert_err!(sub5.apply_to(0));
        assert_err!(sub5.apply_to(1));

        assert_ok_eq!(sub5.apply_to(5), 0);
        assert_ok_eq!(sub5.apply_to(100), 95);
    }

    #[test]
    fn test_delta_merge() {
        use DeltaUpdate::*;

        let mut v = delta_add(5, 20);
        let add20 = delta_add(20, 20);
        let sub15 = delta_sub(15, 20);
        let add7 = delta_add(7, 20);
        let mut sub1 = delta_sub(1, 20);
        let sub20 = delta_sub(20, 20);

        // Overflow on merge.
        assert_err!(v.merge_with(add20)); // 25

        // Successful merges.
        assert_ok!(v.merge_with(v)); // 10
        assert_matches!(v.update, Plus(10));
        assert_ok!(v.merge_with(sub15)); // -5
        assert_matches!(v.update, Minus(5));
        assert_ok!(v.merge_with(add7)); // 2
        assert_matches!(v.update, Plus(2));
        assert_ok!(v.merge_with(sub1)); // 1
        assert_matches!(v.update, Plus(1));

        // Underflow on merge.
        assert_err!(sub1.merge_with(sub20)); // -21
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
        let delta_op = delta_add(10, 1000);
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
        let add_op = delta_add(100, 200);
        let sub_op = delta_sub(100, 200);

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
        let add_op = delta_add(15, 100);
        let sub_op = delta_sub(101, 1000);

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
