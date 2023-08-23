// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op. Every delta is is a state key
//! (for accessing the storage) and an operation: a partial function with a
//! postcondition.

use crate::module::AGGREGATOR_MODULE;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    vm_status::{StatusCode, VMStatus},
    write_set::WriteOp,
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult};

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

        // If delta has been successfully validated, apply the update.
        match self.update {
            DeltaUpdate::Plus(value) => addition(base, value, self.limit),
            DeltaUpdate::Minus(value) => subtraction(base, value),
        }
    }

    /// Shifts by a `delta` the maximum positive value seen by `self`.
    fn shifted_max_positive_by(&self, delta: &DeltaOp) -> PartialVMResult<u128> {
        match delta.update {
            // Suppose that maximum value seen is +M and we shift by +V. Then the
            // new maximum value is M+V provided addition do no overflow.
            DeltaUpdate::Plus(value) => addition(value, self.max_positive, self.limit),
            // Suppose that maximum value seen is +M and we shift by -V this time.
            // If M >= V, the result is +(M-V). Otherwise, `self` should have never
            // reached any positive value. By convention, we use 0 for the latter
            // case. Also, we can reuse `subtraction` which throws an error when M < V,
            // simply mapping the error to 0.
            DeltaUpdate::Minus(value) => Ok(subtraction(self.max_positive, value).unwrap_or(0)),
        }
    }

    /// Shifts by a `delta` the minimum negative value seen by `self`.
    fn shifted_min_negative_by(&self, delta: &DeltaOp) -> PartialVMResult<u128> {
        match delta.update {
            // Suppose that minimum value seen is -M and we shift by +V. Then this case
            // is symmetric to +M-V in `shifted_max_positive_by`. Indeed, if M >= V, then
            // the minimum value should become -(M-V). Otherwise, delta had never been
            // negative and the minimum value capped to 0.
            DeltaUpdate::Plus(value) => Ok(subtraction(self.min_negative, value).unwrap_or(0)),
            // Otherwise, given  the minimum value of -M and the shift of -V the new
            // minimum value becomes -(M+V), which of course can overflow on addition,
            // implying that we subtracted too much and there was an underflow.
            DeltaUpdate::Minus(value) => addition(value, self.min_negative, self.limit),
        }
    }

    /// Applies self on top of previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows
    /// correctly.
    pub fn merge_with_previous_delta(&mut self, previous_delta: DeltaOp) -> PartialVMResult<()> {
        use DeltaUpdate::*;

        assert_eq!(
            self.limit, previous_delta.limit,
            "Cannot merge deltas with different limits",
        );

        // First, update the history values of this delta given that it starts from
        // +value or -value instead of 0. We should do this check to avoid cases like this:
        //
        // Suppose we have deltas with limit of 100, and we have some `d2` which is +3 but it
        // was +99 at some point. Now, if we merge some `d1` which is +2 with `d2`, we get
        // the result is +5. However, it should not have happened because `d2` should hit
        // +2+99 > 100 at some point in history and fail.
        let shifted_max_positive = self.shifted_max_positive_by(&previous_delta)?;
        let shifted_min_negative = self.shifted_min_negative_by(&previous_delta)?;

        // Useful macro for merging deltas of the same sign, e.g. +A+B or -A-B.
        // In this cases we compute the absolute sum of deltas (A+B) and use plus
        // or minus sign accordingly.
        macro_rules! update_same_sign {
            ($sign:ident, $a:ident, $b:ident) => {
                self.update = $sign(addition($a, $b, self.limit)?)
            };
        }

        // Another useful macro, this time for merging deltas with different signs, such
        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
        // and possibly flip a sign.
        macro_rules! update_different_sign {
            ($a:ident, $b:ident) => {
                if $a >= $b {
                    self.update = Plus(subtraction($a, $b)?);
                } else {
                    self.update = Minus(subtraction($b, $a)?);
                }
            };
        }

        // History check passed, and we are ready to update the actual values now.
        match previous_delta.update {
            Plus(prev_value) => match self.update {
                Plus(self_value) => update_same_sign!(Plus, prev_value, self_value),
                Minus(self_value) => update_different_sign!(prev_value, self_value),
            },
            Minus(prev_value) => match self.update {
                Plus(self_value) => update_different_sign!(self_value, prev_value),
                Minus(self_value) => update_same_sign!(Minus, prev_value, self_value),
            },
        }

        // Deltas have been merged successfully - update the history as well.
        self.max_positive = u128::max(previous_delta.max_positive, shifted_max_positive);
        self.min_negative = u128::max(previous_delta.min_negative, shifted_min_negative);
        Ok(())
    }

    /// Applies next delta on top of self, merging two deltas together. This is a reverse
    /// of `merge_with_previous_delta`.
    pub fn merge_with_next_delta(&mut self, next_delta: DeltaOp) -> PartialVMResult<()> {
        // Now self follows the other delta.
        let mut previous_delta = next_delta;
        std::mem::swap(self, &mut previous_delta);

        // Perform the merge.
        self.merge_with_previous_delta(previous_delta)?;
        Ok(())
    }

    /// Consumes a single delta and tries to materialize it with a given state
    /// key. If materialization succeeds, a write op is produced. Otherwise, an
    /// error VM status is returned.
    pub fn try_into_write_op(
        self,
        state_view: &dyn StateView,
        state_key: &StateKey,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        // In case storage fails to fetch the value, return immediately.
        let maybe_value = state_view
            .get_state_value_u128(state_key)
            .map_err(|e| VMStatus::error(StatusCode::STORAGE_ERROR, Some(e.to_string())))?;

        // Otherwise we have to apply delta to the storage value.
        match maybe_value {
            Some(base) => {
                self.apply_to(base)
                    .map_err(|partial_error| {
                        // If delta application fails, transform partial VM
                        // error into an appropriate VM status.
                        partial_error
                            .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                            .into_vm_status()
                    })
                    .map(|result| WriteOp::Modification(serialize(&result)))
            },
            // Something is wrong, the value to which we apply delta should
            // always exist. Guard anyway.
            None => Err(VMStatus::error(
                StatusCode::STORAGE_ERROR,
                Some("Aggregator value does not exist in storage.".to_string()),
            )),
        }
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

/// Error for delta application. Can be used by delta partial functions
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

#[cfg(test)]
mod test {
    use super::*;
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_state_view::TStateView;
    use aptos_types::state_store::{
        state_storage_usage::StateStorageUsage, state_value::StateValue,
    };
    use claims::{assert_err, assert_matches, assert_ok, assert_ok_eq};
    use once_cell::sync::Lazy;

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
        let mut b = delta_add(1, 100);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Plus(3));
        assert_eq!(b.max_positive, 4);
        assert_eq!(b.min_negative, 3);

        // Case 2: updating history upper bound.
        // Explanation: again, value is clearly +3, but this time the upper bound
        // in history is updated with +3+4 > +4, but lower bound is preserved
        // with -3 < +3-4.
        let a = delta_add_with_history(2, 100, 4, 3);
        let mut b = delta_add_with_history(3, 100, 4, 4);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Plus(5));
        assert_eq!(b.max_positive, 6);
        assert_eq!(b.min_negative, 3);

        // Case 3: updating history lower bound.
        // Explanation: clearly, upper bound remains at +90, but lower bound
        // has to be updated with +5-10 < -3.
        let a = delta_add_with_history(5, 100, 90, 3);
        let mut b = delta_add_with_history(10, 100, 4, 10);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Plus(15));
        assert_eq!(b.max_positive, 90);
        assert_eq!(b.min_negative, 5);

        // Case 4: overflow on value.
        // Explanation: value overflows because +51+50 > 100.
        let a = delta_add(51, 100);
        let mut b = delta_add(50, 100);
        let mut c = a;
        let d = b;

        assert_err!(c.merge_with_next_delta(d));
        assert_err!(b.merge_with_previous_delta(a));

        // Case 5: overflow on upper bound in the history.
        // Explanation: the new upper bound would be +5+96 > 100 and should not
        // have happened.
        let a = delta_add_with_history(5, 100, 90, 3);
        let mut b = delta_add_with_history(10, 100, 96, 0);
        let mut c = a;
        let d = b;

        assert_err!(c.merge_with_next_delta(d));
        assert_err!(b.merge_with_previous_delta(a));

        // Case 6: updating value with changing the sign. Note that we do not
        // test history here and onwards, because that code is shared by
        // plus-plus and plus-minus cases.
        // Explanation: +24-23 = +1
        let a = delta_add(24, 100);
        let mut b = delta_sub(23, 100);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Plus(1));

        // Case 7: updating value with changing the sign.
        // Explanation: +23-24 = -1
        let a = delta_add(23, 100);
        let mut b = delta_sub_with_history(24, 100, 20, 20);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(1));
    }

    #[test]
    fn test_delta_merge_minus() {
        use DeltaUpdate::*;

        // Case 1: preserving old history and updating the value.
        // Explanation: value becomes -20-20 = -40, history remains unchanged
        // because +1 > 0 and -40 <= -20-0.
        let a = delta_sub_with_history(20, 100, 1, 40);
        let mut b = delta_sub(20, 100);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(40));
        assert_eq!(b.max_positive, 1);
        assert_eq!(b.min_negative, 40);

        // Case 2: updating history upper bound.
        // Explanation: upper bound is changed because -2+7 > 4. Lower bound
        // remains unchanged because -2-7 > -10.
        let a = delta_sub_with_history(2, 100, 4, 10);
        let mut b = delta_sub_with_history(3, 100, 7, 7);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(5));
        assert_eq!(b.max_positive, 5);
        assert_eq!(b.min_negative, 10);

        // Case 3: updating history lower bound.
        // Explanation: +90 > -5+95 and therefore upper bound remains the same.
        // For lower bound, we have to update it because -5-4 < -5.
        let a = delta_sub_with_history(5, 100, 90, 5);
        let mut b = delta_sub_with_history(10, 100, 95, 4);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(15));
        assert_eq!(b.max_positive, 90);
        assert_eq!(b.min_negative, 9);

        // Case 4: underflow on value.
        // Explanation: value underflows because -50-51 clearly should have
        // never happened.
        let a = delta_sub(50, 100);
        let mut b = delta_sub(51, 100);
        let mut c = a;
        let d = b;

        assert_err!(c.merge_with_next_delta(d));
        assert_err!(b.merge_with_previous_delta(a));

        // Case 5: underflow on lower bound in the history.
        // Explanation: the new lower bound would be -5-96 which clearly underflows.
        let a = delta_sub_with_history(5, 100, 0, 3);
        let mut b = delta_sub_with_history(10, 100, 0, 96);
        let mut c = a;
        let d = b;

        assert_err!(c.merge_with_next_delta(d));
        assert_err!(b.merge_with_previous_delta(a));

        // Case 6: updating value with changing the sign.
        // Explanation: -24+23 = -1.
        let a = delta_sub(24, 100);
        let mut b = delta_add(23, 100);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(1));

        // Case 7: updating value with changing the sign.
        // Explanation: +23-24 = +1.
        let a = delta_add(23, 100);
        let mut b = delta_sub_with_history(24, 100, 20, 20);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Minus(1));
    }

    static KEY: Lazy<StateKey> = Lazy::new(|| StateKey::raw(String::from("test-key").into_bytes()));

    #[test]
    fn test_failed_write_op_conversion_because_of_empty_storage() {
        let state_view = FakeDataStore::default();
        let delta_op = delta_add(10, 1000);
        assert_matches!(
            delta_op.try_into_write_op(&state_view, &KEY),
            Err(VMStatus::Error {
                status_code: StatusCode::STORAGE_ERROR,
                message: Some(_),
                sub_status: None
            })
        );
    }

    struct BadStorage;

    impl TStateView for BadStorage {
        type Key = StateKey;

        fn get_state_value(&self, _state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
            Err(anyhow::Error::new(VMStatus::error(
                StatusCode::STORAGE_ERROR,
                Some("Error message from BadStorage.".to_string()),
            )))
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            unreachable!()
        }
    }

    #[test]
    fn test_failed_write_op_conversion_because_of_storage_error() {
        let state_view = BadStorage;
        let delta_op = delta_add(10, 1000);
        assert_matches!(
            delta_op.try_into_write_op(&state_view, &KEY),
            Err(VMStatus::Error {
                status_code: StatusCode::STORAGE_ERROR,
                message: Some(_),
                sub_status: None
            })
        );
    }

    #[test]
    fn test_successful_write_op_conversion() {
        let mut state_view = FakeDataStore::default();
        state_view.set_legacy(KEY.clone(), serialize(&100));

        // Both addition and subtraction should succeed!
        let add_op = delta_add(100, 200);
        let sub_op = delta_sub(100, 200);

        let add_result = add_op.try_into_write_op(&state_view, &KEY);
        assert_ok_eq!(add_result, WriteOp::Modification(serialize(&200)));

        let sub_result = sub_op.try_into_write_op(&state_view, &KEY);
        assert_ok_eq!(sub_result, WriteOp::Modification(serialize(&0)));
    }

    #[test]
    fn test_unsuccessful_write_op_conversion() {
        let mut state_view = FakeDataStore::default();
        state_view.set_legacy(KEY.clone(), serialize(&100));

        // Both addition and subtraction should fail!
        let add_op = delta_add(15, 100);
        let sub_op = delta_sub(101, 1000);

        assert_matches!(
            add_op.try_into_write_op(&state_view, &KEY),
            Err(VMStatus::MoveAbort(_, EADD_OVERFLOW))
        );
        assert_matches!(
            sub_op.try_into_write_op(&state_view, &KEY),
            Err(VMStatus::MoveAbort(_, ESUB_UNDERFLOW))
        );
    }
}
