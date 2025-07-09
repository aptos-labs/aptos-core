// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Parallel data aggregation uses a `Delta` op. Every delta is is a state key
//! (for accessing the storage) and an operation: a partial function with a
//! postcondition.

use crate::{
    bounded_math::{BoundedMath, SignedU128},
    delta_math::{merge_data_and_delta, merge_two_deltas, DeltaHistory},
    types::{DelayedFieldsSpeculativeError, DeltaApplicationFailureReason},
};
use aptos_types::error::{code_invariant_error, PanicOr};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DeltaWithMax {
    /// Delta which is the result of the execution.
    pub update: SignedU128,
    /// The maximum value the aggregator can reach.
    pub max_value: u128,
}

impl DeltaWithMax {
    /// Creates a new delta op.
    pub fn new(update: SignedU128, max_value: u128) -> Self {
        Self { max_value, update }
    }

    /// Returns the kind of update for the delta op.
    pub fn get_update(&self) -> SignedU128 {
        self.update
    }

    /// Returns the result of delta application to `base` or error if
    /// postcondition is not satisfied.
    pub fn apply_to(&self, base: u128) -> Result<u128, DelayedFieldsSpeculativeError> {
        let math = BoundedMath::new(self.max_value);
        match self.update {
            SignedU128::Positive(value) => math.unsigned_add(base, value).map_err(|_e| {
                DelayedFieldsSpeculativeError::DeltaApplication {
                    base_value: base,
                    max_value: self.max_value,
                    delta: self.update,
                    reason: DeltaApplicationFailureReason::Overflow,
                }
            }),
            SignedU128::Negative(value) => math.unsigned_subtract(base, value).map_err(|_e| {
                DelayedFieldsSpeculativeError::DeltaApplication {
                    base_value: base,
                    max_value: self.max_value,
                    delta: self.update,
                    reason: DeltaApplicationFailureReason::Underflow,
                }
            }),
        }
    }

    pub fn create_merged_delta(
        prev_delta: &DeltaWithMax,
        next_delta: &DeltaWithMax,
    ) -> Result<DeltaWithMax, PanicOr<DelayedFieldsSpeculativeError>> {
        if prev_delta.max_value != next_delta.max_value {
            Err(code_invariant_error(
                "Cannot merge deltas with different limits",
            ))?;
        }

        let new_delta = BoundedMath::new(prev_delta.max_value)
            .signed_add(&prev_delta.update, &next_delta.update)
            .map_err(|_| DelayedFieldsSpeculativeError::DeltaMerge {
                base_delta: prev_delta.update,
                delta: next_delta.update,
                max_value: prev_delta.max_value,
            })?;

        Ok(DeltaWithMax::new(new_delta, prev_delta.max_value))
    }

    pub fn into_op_no_additional_history(self) -> DeltaOp {
        let mut history = DeltaHistory::new();
        history.record_success(self.update);
        DeltaOp::new(self.update, self.max_value, history)
    }
}

/// Represents an update from aggregator's operation.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DeltaOp {
    /// History computed during the transaction execution.
    history: DeltaHistory,
    /// The maximum value the aggregator can reach.
    max_value: u128,
    /// Delta which is the result of the execution.
    update: SignedU128,
}

impl DeltaOp {
    /// Creates a new delta op.
    pub fn new(update: SignedU128, max_value: u128, history: DeltaHistory) -> Self {
        Self {
            history,
            max_value,
            update,
        }
    }

    /// Returns the kind of update for the delta op.
    pub fn get_update(&self) -> SignedU128 {
        self.update
    }

    /// Returns the result of delta application to `base` or error if
    /// postcondition is not satisfied.
    pub fn apply_to(&self, base: u128) -> Result<u128, PanicOr<DelayedFieldsSpeculativeError>> {
        merge_data_and_delta(base, &self.update, &self.history, self.max_value)
    }

    pub fn create_merged_delta(
        prev_delta: &DeltaOp,
        next_delta: &DeltaOp,
    ) -> Result<DeltaOp, PanicOr<DelayedFieldsSpeculativeError>> {
        if prev_delta.max_value != next_delta.max_value {
            Err(code_invariant_error(
                "Cannot merge deltas with different limits",
            ))?;
        }

        let (new_update, new_history) = merge_two_deltas(
            &prev_delta.update,
            &prev_delta.history,
            &next_delta.update,
            &next_delta.history,
            next_delta.max_value,
        )?;

        Ok(DeltaOp::new(new_update, next_delta.max_value, new_history))
    }

    /// Applies self on top of previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows
    /// correctly.
    pub fn merge_with_previous_delta(
        &mut self,
        previous_delta: DeltaOp,
    ) -> Result<(), PanicOr<DelayedFieldsSpeculativeError>> {
        *self = Self::create_merged_delta(&previous_delta, self)?;
        Ok(())
    }

    /// Applies next delta on top of self, merging two deltas together. This is a reverse
    /// of `merge_with_previous_delta`.
    pub fn merge_with_next_delta(
        &mut self,
        next_delta: DeltaOp,
    ) -> Result<(), PanicOr<DelayedFieldsSpeculativeError>> {
        *self = Self::create_merged_delta(self, &next_delta)?;
        Ok(())
    }

    pub fn into_inner(self) -> (SignedU128, DeltaHistory, u128) {
        (self.update, self.history, self.max_value)
    }
}

impl std::fmt::Debug for DeltaOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.update {
            SignedU128::Positive(value) => {
                write!(
                    f,
                    "+{} ensures 0 <= result <= {}, {:?}",
                    value, self.max_value, self.history
                )
            },
            SignedU128::Negative(value) => {
                write!(
                    f,
                    "-{} ensures 0 <= result <= {}, {:?}",
                    value, self.max_value, self.history
                )
            },
        }
    }
}

/// Serializes value after delta application.
pub fn serialize(value: &u128) -> Vec<u8> {
    bcs::to_bytes(value).expect("unexpected serialization error in aggregator")
}

#[cfg(any(test, feature = "testing"))]
pub fn delta_sub(v: u128, max_value: u128) -> DeltaOp {
    DeltaOp::new(SignedU128::Negative(v), max_value, DeltaHistory {
        max_achieved_positive_delta: 0,
        min_achieved_negative_delta: v,
        min_overflow_positive_delta: None,
        max_underflow_negative_delta: None,
    })
}

#[cfg(any(test, feature = "testing"))]
pub fn delta_add(v: u128, max_value: u128) -> DeltaOp {
    DeltaOp::new(SignedU128::Positive(v), max_value, DeltaHistory {
        max_achieved_positive_delta: v,
        min_achieved_negative_delta: 0,
        min_overflow_positive_delta: None,
        max_underflow_negative_delta: None,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        aggregator_v1_extension::{EADD_OVERFLOW, ESUB_UNDERFLOW},
        resolver::{TAggregatorV1View, TDelayedFieldView},
        types::DelayedFieldValue,
        FakeAggregatorView,
    };
    use aptos_types::{
        error::PanicError,
        state_store::{
            state_key::StateKey,
            state_value::{StateValue, StateValueMetadata},
        },
        write_set::WriteOp,
    };
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some_eq};
    use move_binary_format::errors::{PartialVMError, PartialVMResult};
    use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
    use once_cell::sync::Lazy;
    use std::{
        collections::{BTreeMap, HashSet},
        sync::Arc,
    };

    fn delta_add_with_history(v: u128, max_value: u128, max: u128, min: u128) -> DeltaOp {
        let mut delta = delta_add(v, max_value);
        delta.history.max_achieved_positive_delta = max;
        delta.history.min_achieved_negative_delta = min;
        delta
    }

    fn delta_sub_with_history(v: u128, max_value: u128, max: u128, min: u128) -> DeltaOp {
        let mut delta = delta_sub(v, max_value);
        delta.history.max_achieved_positive_delta = max;
        delta.history.min_achieved_negative_delta = min;
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
        add5.history.max_achieved_positive_delta = 50;
        add5.history.min_achieved_negative_delta = 10;
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
        sub5.history.max_achieved_positive_delta = 10;
        sub5.history.min_achieved_negative_delta = 20;
        assert_err!(sub5.apply_to(19)); // underflow: 19 - 20 < 0!
        assert_err!(sub5.apply_to(91)); // overflow:  91 + 10 > 100!
        assert_ok_eq!(sub5.apply_to(20), 15);
        assert_ok_eq!(sub5.apply_to(90), 85);
    }

    #[test]
    fn test_delta_merge_plus() {
        use SignedU128::*;

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
        assert_eq!(b.update, Positive(3));
        assert_eq!(b.history.max_achieved_positive_delta, 4);
        assert_eq!(b.history.min_achieved_negative_delta, 3);

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
        assert_eq!(b.update, Positive(5));
        assert_eq!(b.history.max_achieved_positive_delta, 6);
        assert_eq!(b.history.min_achieved_negative_delta, 3);

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
        assert_eq!(b.update, Positive(15));
        assert_eq!(b.history.max_achieved_positive_delta, 90);
        assert_eq!(b.history.min_achieved_negative_delta, 5);

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
        assert_eq!(b.update, Positive(1));

        // Case 7: updating value with changing the sign.
        // Explanation: +23-24 = -1
        let a = delta_add(23, 100);
        let mut b = delta_sub_with_history(24, 100, 20, 20);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Negative(1));
    }

    #[test]
    fn test_delta_merge_minus() {
        use SignedU128::*;

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
        assert_eq!(b.update, Negative(40));
        assert_eq!(b.history.max_achieved_positive_delta, 1);
        assert_eq!(b.history.min_achieved_negative_delta, 40);

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
        assert_eq!(b.update, Negative(5));
        assert_eq!(b.history.max_achieved_positive_delta, 5);
        assert_eq!(b.history.min_achieved_negative_delta, 10);

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
        assert_eq!(b.update, Negative(15));
        assert_eq!(b.history.max_achieved_positive_delta, 90);
        assert_eq!(b.history.min_achieved_negative_delta, 9);

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
        assert_eq!(b.update, Negative(1));

        // Case 7: updating value with changing the sign.
        // Explanation: +23-24 = +1.
        let a = delta_add(23, 100);
        let mut b = delta_sub_with_history(24, 100, 20, 20);
        let mut c = a;
        let d = b;

        assert_ok!(b.merge_with_previous_delta(a));
        assert_ok!(c.merge_with_next_delta(d));
        assert_eq!(b, c);
        assert_eq!(b.update, Negative(1));
    }

    static KEY: Lazy<StateKey> = Lazy::new(|| StateKey::raw(b"test-key"));

    #[test]
    fn test_failed_write_op_conversion_because_of_empty_storage() {
        let state_view = FakeAggregatorView::default();
        let delta_op = delta_add(10, 1000);

        let err =
            assert_err!(state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &delta_op));
        assert_eq!(
            err.major_status(),
            StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
        );
        assert_none!(err.sub_status());
    }

    struct BadStorage;

    impl TAggregatorV1View for BadStorage {
        type Identifier = StateKey;

        fn get_aggregator_v1_state_value(
            &self,
            _id: &Self::Identifier,
        ) -> PartialVMResult<Option<StateValue>> {
            Err(
                PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                    .with_message("Error message from BadStorage.".to_string()),
            )
        }
    }

    impl TDelayedFieldView for BadStorage {
        type Identifier = ();
        type ResourceGroupTag = ();
        type ResourceKey = ();

        fn get_delayed_field_value(
            &self,
            _id: &Self::Identifier,
        ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
            Err(code_invariant_error("Error message from BadStorage.").into())
        }

        fn delayed_field_try_add_delta_outcome(
            &self,
            _id: &Self::Identifier,
            _base_delta: &SignedU128,
            _delta: &SignedU128,
            _max_value: u128,
        ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
            Err(code_invariant_error("Error message from BadStorage.").into())
        }

        fn generate_delayed_field_id(&self, _width: u32) -> Self::Identifier {
            unimplemented!("Irrelevant for the test")
        }

        fn validate_delayed_field_id(&self, _id: &Self::Identifier) -> Result<(), PanicError> {
            unimplemented!("Irrelevant for the test")
        }

        fn get_reads_needing_exchange(
            &self,
            _delayed_write_set_keys: &HashSet<Self::Identifier>,
            _skip: &HashSet<Self::ResourceKey>,
        ) -> Result<
            BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
            PanicError,
        > {
            unimplemented!("Irrelevant for the test")
        }

        fn get_read_needing_exchange(
            &self,
            _key: &Self::ResourceKey,
            _delayed_write_set_ids: &HashSet<Self::Identifier>,
        ) -> Result<Option<(StateValueMetadata, u64)>, PanicError> {
            unimplemented!("Irrelevant for the test")
        }

        fn get_group_reads_needing_exchange(
            &self,
            _delayed_write_set_keys: &HashSet<Self::Identifier>,
            _skip: &HashSet<Self::ResourceKey>,
        ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
            unimplemented!("Irrelevant for the test")
        }

        fn get_group_read_needing_exchange(
            &self,
            _key: &Self::ResourceKey,
            _delayed_write_set_ids: &HashSet<Self::Identifier>,
        ) -> PartialVMResult<Option<(StateValueMetadata, u64)>> {
            unimplemented!("Irrelevant for the test")
        }
    }

    #[test]
    fn test_failed_write_op_conversion_because_of_speculative_error() {
        let state_view = BadStorage;
        let delta_op = delta_add(10, 1000);

        let err =
            assert_err!(state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &delta_op));
        assert_eq!(
            err.major_status(),
            StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
        );
        assert_none!(err.sub_status());
    }

    #[test]
    fn test_successful_write_op_conversion() {
        let mut state_view = FakeAggregatorView::default();
        state_view.set_from_state_key(KEY.clone(), 100);

        // Both addition and subtraction should succeed!
        let add_op = delta_add(100, 200);
        let sub_op = delta_sub(100, 200);

        let add_result = state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &add_op);
        assert_ok_eq!(
            add_result,
            WriteOp::legacy_modification(serialize(&200).into())
        );

        let sub_result = state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &sub_op);
        assert_ok_eq!(
            sub_result,
            WriteOp::legacy_modification(serialize(&0).into())
        );
    }

    #[test]
    fn test_unsuccessful_write_op_conversion() {
        let mut state_view = FakeAggregatorView::default();
        state_view.set_from_state_key(KEY.clone(), 100);

        // Both addition and subtraction should fail!
        let add_op = delta_add(15, 100);
        let sub_op = delta_sub(101, 1000);

        let err =
            assert_err!(state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &add_op));
        assert_eq!(err.major_status(), StatusCode::ABORTED);
        assert_some_eq!(err.sub_status(), EADD_OVERFLOW);

        let err =
            assert_err!(state_view.try_convert_aggregator_v1_delta_into_write_op(&KEY, &sub_op));
        assert_eq!(err.major_status(), StatusCode::ABORTED);
        assert_some_eq!(err.sub_status(), ESUB_UNDERFLOW);
    }
}
