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
    /// Delta which is the result of the execution.
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

    /// Applies self on top of previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows
    /// correctly.
    pub fn merge_onto(&mut self, previous_delta: DeltaOp) -> PartialVMResult<()> {
        use DeltaUpdate::*;

        Ok(match previous_delta.update {
            Plus(prev_value) => {
                // Before we proceed to merging deltas, make sure we verify that
                // `self` delta did not overflow when shifted by +value. At the
                // same time, we can compute new history values for `self` delta.
                let shifted_max_positive = addition(prev_value, self.max_positive, self.limit)?;
                let shifted_min_negative = subtraction(self.min_negative, prev_value).unwrap_or(0);

                // If check has passed, update the value.
                match self.update {
                    Plus(self_value) => {
                        let new_value = addition(prev_value, self_value, self.limit)?;
                        self.update = Plus(new_value);
                    }
                    Minus(self_value) => {
                        if prev_value >= self_value {
                            let new_value = subtraction(prev_value, self_value)?;
                            self.update = Plus(new_value);
                        } else {
                            let new_value = subtraction(self_value, prev_value)?;
                            self.update = Minus(new_value);
                        };
                    }
                }

                // Lastly, update the history.
                self.max_positive = previous_delta.max_positive.max(shifted_max_positive);
                self.min_negative = previous_delta.min_negative.max(shifted_min_negative);
            }
            Minus(prev_value) => {
                // Again, first we verify that the merging makes sense at all.
                // Now, we can underflow if the minimum value of `self` drops
                // too much (i.e. when applying -value).
                let shifted_min_negative = addition(prev_value, self.min_negative, self.limit)?;
                let shifted_max_positive = subtraction(self.max_positive, prev_value).unwrap_or(0);

                // Update the value and history.
                match self.update {
                    Plus(self_value) => {
                        if self_value >= prev_value {
                            let new_value = subtraction(self_value, prev_value)?;
                            self.update = Plus(new_value);
                        } else {
                            let new_value = subtraction(prev_value, self_value)?;
                            self.update = Minus(new_value);
                        };
                    }
                    Minus(self_value) => {
                        let new_value = addition(prev_value, self_value, self.limit)?;
                        self.update = Minus(new_value);
                    }
                }
                self.max_positive = previous_delta.max_positive.max(shifted_max_positive);
                self.min_negative = previous_delta.min_negative.max(shifted_min_negative);
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

    fn delta_add_with_history(v: u128, limit: u128, max: u128, min: u128) -> DeltaOp {
        let mut delta = delta_add(v, limit);
        delta.max_positive = max;
        delta.min_negative = min;
        delta
    }

    fn delta_sub_with_history(v: u128, limit: u128, max: u128, min: u128) -> DeltaOp {
        let mut delta = delta_sub(v, limit);
        delta.max_positive = max;
        delta.min_negative = min;
        delta
    }

    #[test]
    fn test_delta_application() {
        // Testing a fresh delta of +5.
        let mut add5 = delta_add(5, 100);
        assert_ok_eq!(add5.apply_to(0), 5);
        assert_ok_eq!(add5.apply_to(95), 100);
        assert_err!(add5.apply_to(96));

        // Testing a delta of +5 with history now. We should consider three
        // cases: underflow, overflow, and successful application.
        add5.max_positive = 50;
        add5.min_negative = 10;
        assert_err!(add5.apply_to(5)); // underflow: 5 - 10 < 0!
        assert_err!(add5.apply_to(51)); // overflow: 51 + 50 > 100!
        assert_ok_eq!(add5.apply_to(10), 15);
        assert_ok_eq!(add5.apply_to(50), 55);

        // Testing a fresh delta of -5.
        let mut sub5 = delta_sub(5, 100);
        assert_ok_eq!(sub5.apply_to(5), 0);
        assert_ok_eq!(sub5.apply_to(100), 95);
        assert_err!(sub5.apply_to(0));
        assert_err!(sub5.apply_to(4));

        // Now, similarly to addition test, update the delta with
        // some random history. Again, we have three cases to check.
        sub5.max_positive = 10;
        sub5.min_negative = 20;
        assert_err!(sub5.apply_to(19)); // underflow: 19 - 20 < 0!
        assert_err!(sub5.apply_to(91)); // overflow:  91 + 10 > 100!
        assert_ok_eq!(sub5.apply_to(20), 15);
        assert_ok_eq!(sub5.apply_to(90), 85);
    }

    #[test]
    fn test_delta_merge_plus() {
        use DeltaUpdate::*;

        // Case 1: preserving old history and updating the value.
        // Explanation: value becomes +2+1 = +3, history remains unchanged
        // because +4 > +2+1 and -3 < 0.
        let a = delta_add_with_history(2, 100, 4, 3);
        let mut d = delta_add(1, 100);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Plus(3));
        assert_eq!(d.max_positive, 4);
        assert_eq!(d.min_negative, 3);

        // Case 2: updating history upper bound.
        // Explanation: again, value is clearly +3, but this time the upper bound
        // in history is updated with +3+4 > +4, but lower bound is preserved
        // with -3 < +3-4.
        let a = delta_add_with_history(2, 100, 4, 3);
        let mut d = delta_add_with_history(3, 100, 4, 4);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Plus(5));
        assert_eq!(d.max_positive, 6);
        assert_eq!(d.min_negative, 3);

        // Case 3: updating history lower bound.
        // Explanation: clearly, upper bound remains at +90, but lower bound
        // has to be updated with +5-10 < -3.
        let a = delta_add_with_history(5, 100, 90, 3);
        let mut d = delta_add_with_history(10, 100, 4, 10);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Plus(15));
        assert_eq!(d.max_positive, 90);
        assert_eq!(d.min_negative, 5);

        // Case 4: overflow on value.
        // Explanation: value overflows because +51+50 > 100.
        let a = delta_add(51, 100);
        let mut d = delta_add(50, 100);
        assert_err!(d.merge_onto(a));

        // Case 5: overflow on upper bound in the history.
        // Explanation: the new upper bound would be +5+96 > 100 and should not
        // have happened.
        let a = delta_add_with_history(5, 100, 90, 3);
        let mut d = delta_add_with_history(10, 100, 96, 0);
        assert_err!(d.merge_onto(a));

        // Case 6: updating value with changing the sign. Note that we do not
        // test history here and onwards, because that code is shared by
        // plus-plus and plus-minus cases.
        // Explanation: +24-23 = +1
        let a = delta_add(24, 100);
        let mut d = delta_sub(23, 100);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Plus(1));

        // Case 7: updating value with changing the sign.
        // Explanation: +23-24 = -1
        let a = delta_add(23, 100);
        let mut d = delta_sub_with_history(24, 100, 20, 20);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Minus(1));
    }

    #[test]
    fn test_delta_merge_minus() {
        use DeltaUpdate::*;

        // Case 1: preserving old history and updating the value.
        // Explanation: value becomes -20-20 = -40, history remains unchanged
        // because +1 > 0 and -40 <= -20-0.
        let a = delta_sub_with_history(20, 100, 1, 40);
        let mut d = delta_sub(20, 100);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Minus(40));
        assert_eq!(d.max_positive, 1);
        assert_eq!(d.min_negative, 40);

        // Case 2: updating history upper bound.
        // Explanation: upper bound is changed because -2+7 > 4. Lower bound
        // remains unchanged because -2-7 > -10.
        let a = delta_sub_with_history(2, 100, 4, 10);
        let mut d = delta_sub_with_history(3, 100, 7, 7);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Minus(5));
        assert_eq!(d.max_positive, 5);
        assert_eq!(d.min_negative, 10);

        // Case 3: updating history lower bound.
        // Explanation: +90 > -5+95 and therefore upper bound remains the same.
        // For lower bound, we have to update it because -5-4 < -5.
        let a = delta_sub_with_history(5, 100, 90, 5);
        let mut d = delta_sub_with_history(10, 100, 95, 4);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Minus(15));
        assert_eq!(d.max_positive, 90);
        assert_eq!(d.min_negative, 9);

        // Case 4: underflow on value.
        // Explanation: value underflows because -50-51 clearly should have
        // never happened.
        let a = delta_sub(50, 100);
        let mut d = delta_sub(51, 100);
        assert_err!(d.merge_onto(a));

        // Case 5: underflow on lower bound in the history.
        // Explanation: the new lower bound would be -5-96 which clearly underflows.
        let a = delta_sub_with_history(5, 100, 0, 3);
        let mut d = delta_sub_with_history(10, 100, 0, 96);
        assert_err!(d.merge_onto(a));

        // Case 6: updating value with changing the sign.
        // Explanation: -24+23 = -1.
        let a = delta_sub(24, 100);
        let mut d = delta_add(23, 100);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Minus(1));

        // Case 7: updating value with changing the sign.
        // Explanation: -23+24 = +1.
        let mut d = delta_sub_with_history(23, 100, 20, 20);
        let a = delta_add(24, 100);
        assert_ok!(d.merge_onto(a));
        assert_eq!(d.update, Plus(1));
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
