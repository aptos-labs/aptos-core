// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::{BoundedMath, SignedU128},
    delta_math::DeltaHistory,
    resolver::AggregatorV1Resolver,
    types::{DelayedFieldsSpeculativeError, DeltaApplicationFailureReason},
};
use aptos_types::{
    error::expect_ok,
    state_store::{state_key::StateKey, table::TableHandle},
    PeerId,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_extensions::ValueHistory;
use smallvec::{smallvec, SmallVec};
use std::collections::{btree_map::Entry, BTreeMap};

/// When `Addition` operation overflows the `limit`.
pub(crate) const EADD_OVERFLOW: u64 = 0x02_0001;

/// When `Subtraction` operation goes below zero.
pub(crate) const ESUB_UNDERFLOW: u64 = 0x02_0002;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AggregatorID(pub StateKey);

impl AggregatorID {
    pub fn new(handle: TableHandle, key: PeerId) -> Self {
        let state_key = StateKey::table_item(&handle, key.as_ref());
        AggregatorID(state_key)
    }

    pub fn as_state_key(&self) -> &StateKey {
        &self.0
    }

    pub fn into_state_key(self) -> StateKey {
        self.0
    }
}

/// Describes the state of each aggregator instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregatorState {
    // If aggregator stores a known value.
    Data,
    // If aggregator stores a non-negative delta.
    PositiveDelta,
    // If aggregator stores a negative delta.
    NegativeDelta,
}

/// Internal aggregator data structure.
#[derive(Debug, Clone)]
pub struct Aggregator {
    // Describes a value of an aggregator.
    value: u128,
    // Describes a state of an aggregator.
    state: AggregatorState,
    // Describes an upper bound of an aggregator. If `value` exceeds it, the
    // aggregator overflows.
    // TODO: Currently this is a single u128 value since we use 0 as a trivial
    // lower bound. If we want to support custom lower bounds, or have more
    // complex postconditions, we should factor this out in its own struct.
    max_value: u128,
    // Describes values seen by this aggregator. Note that if aggregator knows
    // its value, then storing history doesn't make sense.
    history: Option<DeltaHistory>,
}

impl Aggregator {
    /// Records observed delta in history. Should be called after an operation
    /// to record its side-effects.
    fn record(&mut self) {
        if let Some(history) = self.history.as_mut() {
            match self.state {
                AggregatorState::PositiveDelta => {
                    history.record_success(SignedU128::Positive(self.value))
                },
                AggregatorState::NegativeDelta => {
                    history.record_success(SignedU128::Negative(self.value))
                },
                AggregatorState::Data => {
                    unreachable!("history is not tracked when aggregator knows its value")
                },
            }
        }
    }

    /// Validates if aggregator's history is correct when applied to
    /// the `base_value`. For example, if history observed a delta of
    /// +100, and the aggregator max_value is 150, then the base value of
    /// 60 will not pass validation (60 + 100 > 150), but the base value
    /// of 30 will (30 + 100 < 150).
    fn validate_history(&self, base_value: u128) -> PartialVMResult<()> {
        let history = self
            .history
            .as_ref()
            .expect("History should be set for validation");

        // To validate the history of an aggregator, we want to ensure
        // that there was no violation of postcondition (i.e. overflows or
        // underflows). We can do it by emulating addition and subtraction.

        if let Err(e) = history.validate_against_base_value(base_value, self.max_value) {
            match e {
                DelayedFieldsSpeculativeError::DeltaApplication {
                    reason: DeltaApplicationFailureReason::Overflow,
                    ..
                } => {
                    return Err(abort_error("overflow", EADD_OVERFLOW));
                },
                DelayedFieldsSpeculativeError::DeltaApplication {
                    reason: DeltaApplicationFailureReason::Underflow,
                    ..
                } => {
                    return Err(abort_error("underflow", ESUB_UNDERFLOW));
                },
                _ => Err(e)?,
            }
        }

        Ok(())
    }

    /// Implements logic for adding to an aggregator.
    pub fn add(&mut self, value: u128) -> PartialVMResult<()> {
        let math = BoundedMath::new(self.max_value);
        match self.state {
            AggregatorState::Data => {
                // If aggregator knows the value, add directly and keep the state.
                self.value = math
                    .unsigned_add(self.value, value)
                    .map_err(addition_v1_error)?;
                return Ok(());
            },
            AggregatorState::PositiveDelta => {
                // If positive delta, add directly but also record the state.
                self.value = math
                    .unsigned_add(self.value, value)
                    .map_err(addition_v1_error)?;
            },
            AggregatorState::NegativeDelta => {
                // Negative delta is a special case, since the state might
                // change depending on how big the `value` is. Suppose
                // aggregator has -X and want to do +Y. Then, there are two
                // cases:
                //     1. X <= Y: then the result is +(Y-X)
                //     2. X  > Y: then the result is -(X-Y)
                if self.value <= value {
                    self.value = expect_ok(math.unsigned_subtract(value, self.value))?;
                    self.state = AggregatorState::PositiveDelta;
                } else {
                    self.value = expect_ok(math.unsigned_subtract(self.value, value))?;
                }
            },
        }

        // Record side-effects of addition in history.
        self.record();
        Ok(())
    }

    /// Implements logic for subtracting from an aggregator.
    pub fn sub(&mut self, value: u128) -> PartialVMResult<()> {
        let math = BoundedMath::new(self.max_value);
        match self.state {
            AggregatorState::Data => {
                // Aggregator knows the value, therefore we can subtract
                // checking we don't drop below zero. We do not need to
                // record the history.
                self.value = math
                    .unsigned_subtract(self.value, value)
                    .map_err(subtraction_v1_error)?;
                return Ok(());
            },
            AggregatorState::PositiveDelta => {
                // Positive delta is a special case because the state can
                // change depending on how big the `value` is. Suppose
                // aggregator has +X and want to do -Y. Then, there are two
                // cases:
                //     1. X >= Y: then the result is +(X-Y)
                //     2. X  < Y: then the result is -(Y-X)
                if self.value >= value {
                    self.value = math
                        .unsigned_subtract(self.value, value)
                        .map_err(subtraction_v1_error)?;
                } else {
                    // Check that we can subtract in general: we don't want to
                    // allow -10000 when max_value is 10.
                    // TODO: maybe `subtraction` should also know about the max_value?
                    math.unsigned_subtract(self.max_value, value)
                        .map_err(subtraction_v1_error)?;

                    self.value = math
                        .unsigned_subtract(value, self.value)
                        .map_err(subtraction_v1_error)?;
                    self.state = AggregatorState::NegativeDelta;
                }
            },
            AggregatorState::NegativeDelta => {
                // Since we operate on unsigned integers, we have to add
                // when subtracting from negative delta. Note that if max_value
                // is some X, then we cannot subtract more than X, and so
                // we should return an error there.
                self.value = math
                    .unsigned_add(self.value, value)
                    .map_err(subtraction_v1_error)?;
            },
        }

        // Record side-effects of addition in history.
        self.record();
        Ok(())
    }

    /// Implements logic for reading the value of an aggregator. As a
    /// result, the aggregator knows it value (i.e. its state changes to
    /// `Data`).
    pub fn read_and_materialize(
        &mut self,
        resolver: &dyn AggregatorV1Resolver,
        id: &AggregatorID,
    ) -> PartialVMResult<u128> {
        // If aggregator has already been read, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(self.value);
        }

        // Otherwise, we have a delta and have to go to storage and apply it.
        // In theory, any delta will be applied to existing value. However,
        // something may go wrong, so we guard by throwing an error in
        // extension.
        let value_from_storage = resolver
            .get_aggregator_v1_value(&id.0)
            .map_err(|e| {
                extension_error(format!("Could not find the value of the aggregator: {}", e))
            })?
            .ok_or_else(|| {
                extension_error(format!(
                    "Could not read from deleted aggregator at {:?}",
                    id
                ))
            })?;

        // Validate history and apply the delta.
        self.validate_history(value_from_storage)?;
        let math = BoundedMath::new(self.max_value);
        match self.state {
            AggregatorState::PositiveDelta => {
                self.value = math
                    .unsigned_add(value_from_storage, self.value)
                    .expect("Validated delta cannot overflow");
            },
            AggregatorState::NegativeDelta => {
                self.value = math
                    .unsigned_subtract(value_from_storage, self.value)
                    .expect("Validated delta cannot underflow");
            },
            AggregatorState::Data => {
                unreachable!("Materialization only happens in Delta state")
            },
        }

        // Change the state and return the new value. Also, make
        // sure history is no longer tracked.
        self.state = AggregatorState::Data;
        self.history = None;
        Ok(self.value)
    }

    /// Unpacks aggregator into its fields.
    pub fn into(self) -> (u128, AggregatorState, u128, Option<DeltaHistory>) {
        (self.value, self.state, self.max_value, self.history)
    }
}

/// Stores all information about aggregators (how many have been created or
/// removed), what are their states, etc. per single transaction).
pub struct AggregatorData {
    next_version: u32,
    saved_versions: SmallVec<[u32; 2]>,
    current_version: u32,
    // All aggregator instances that exist in the current transaction.
    aggregators: BTreeMap<AggregatorID, ValueHistory<Option<Aggregator>>>,
    aggregators_count: u32,
}

impl AggregatorData {
    pub fn undo(&mut self) {
        if self.saved_versions.len() > 1 {
            self.saved_versions.pop();
            self.current_version = *self
                .saved_versions
                .last()
                .expect("Saved version must exist");
        }
    }

    pub fn save(&mut self) {
        self.current_version = self.next_version;
        self.saved_versions.push(self.current_version);
        self.next_version += 1;
    }

    pub fn update(&mut self) {
        self.aggregators_count = 0;
    }

    /// Returns a mutable reference to an aggregator with `id` and a `max_value`.
    /// If transaction that is currently executing did not initialize it, a new aggregator instance is created.
    /// Note: when we say "aggregator instance" here we refer to Rust struct and
    /// not to the Move aggregator.
    pub fn get_aggregator(
        &mut self,
        id: AggregatorID,
        max_value: u128,
    ) -> PartialVMResult<&mut Aggregator> {
        Ok(match self.aggregators.entry(id) {
            Entry::Vacant(entry) => {
                let mut value = ValueHistory::new();
                value.set(
                    self.current_version,
                    Some(Aggregator {
                        value: 0,
                        state: AggregatorState::PositiveDelta,
                        max_value,
                        history: Some(DeltaHistory::new()),
                    }),
                );

                let h = entry.insert(value);
                h.last_mut(self.current_version)
                    .expect("Aggregator value must be set")
                    .as_mut()
                    .expect("Aggregator was just created")
            },
            Entry::Occupied(entry) => {
                let e = entry.into_mut();
                e.last_mut(self.current_version)
                    .expect("If history exists, there should be at least one value")
                    .as_mut()
                    .ok_or_else(|| {
                        PartialVMError::new_invariant_violation(
                            "Cannot request an aggregator if it was deleted",
                        )
                    })?
            },
        })
    }

    /// Returns the number of aggregators that are used in the current transaction.
    pub fn aggregator_count(&self) -> u32 {
        self.aggregators_count
    }

    /// Creates and a new Aggregator with a given `id` and a `max_value`. The value
    /// of a new aggregator is always known, therefore it is created in a data
    /// state, with a zero-initialized value.
    pub fn create_new_aggregator(&mut self, id: AggregatorID, max_value: u128) {
        let aggregator = Aggregator {
            value: 0,
            state: AggregatorState::Data,
            max_value,
            history: None,
        };

        self.aggregators
            .entry(id.clone())
            .or_insert_with(ValueHistory::new)
            .set(self.current_version, Some(aggregator));
        self.aggregators_count += 1;
    }

    /// If aggregator has been used in this transaction, it is removed. Otherwise,
    /// it is marked for deletion.
    pub fn remove_aggregator(&mut self, id: AggregatorID) -> PartialVMResult<()> {
        match self.aggregators.entry(id) {
            Entry::Vacant(entry) => {
                let mut value = ValueHistory::new();
                value.set(self.current_version, None);
                entry.insert(value);
            },
            Entry::Occupied(entry) => {
                if entry
                    .into_mut()
                    .last_mut(self.current_version)
                    .expect("If history exists, there should be at least one value")
                    .take()
                    .is_none()
                {
                    return Err(PartialVMError::new_invariant_violation(
                        "Aggregator is already removed",
                    ));
                }
            },
        }
        Ok(())
    }

    /// Unpacks aggregator data.
    pub fn into(self) -> BTreeMap<AggregatorID, Option<Aggregator>> {
        let current_version = self.current_version;

        self.aggregators
            .into_iter()
            .map(|(id, h)| {
                (
                    id,
                    h.into_last(current_version)
                        .expect("At least one value must always exist"),
                )
            })
            .collect()
    }
}

impl Default for AggregatorData {
    fn default() -> Self {
        Self {
            next_version: 1,
            saved_versions: smallvec![0],
            current_version: 0,
            aggregators: BTreeMap::new(),
            aggregators_count: 0,
        }
    }
}

pub(crate) fn addition_v1_error<T>(_err: T) -> PartialVMError {
    abort_error("overflow", EADD_OVERFLOW)
}

pub(crate) fn subtraction_v1_error<T>(_err: T) -> PartialVMError {
    abort_error("underflow", ESUB_UNDERFLOW)
}

/// Error for delta application. Can be used by delta partial functions
/// to return descriptive error messages and an appropriate error code.
fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

/// Returns partial VM error on extension failure.
pub fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::{aggregator_v1_id_for_test, FakeAggregatorView};
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_materialize_not_in_storage() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(300), 700)
            .expect("Get aggregator failed");
        assert_err!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(700)));
    }

    #[test]
    fn test_materialize_known() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(200), 200)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.add(100));
        assert_ok!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(200)));
        assert_eq!(aggregator.value, 100);
    }

    #[test]
    fn test_materialize_overflow() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();

        // +0 to +400 satisfies <= 600 and is ok, but materialization fails
        // with 300 + 400 > 600!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.add(400));
        assert_err!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(600)));
    }

    #[test]
    fn test_materialize_underflow() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();

        // +0 to -400 is ok, but materialization fails with 300 - 400 < 0!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.add(400));
        assert_err!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(600)));
    }

    #[test]
    fn test_materialize_non_monotonic_1() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();

        // +0 to +400 to +0 is ok, but materialization fails since we had 300 + 400 > 600!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.add(400));
        assert_ok!(aggregator.sub(300));
        assert_eq!(aggregator.value, 100);
        assert_eq!(aggregator.state, AggregatorState::PositiveDelta);
        assert_err!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(600)));
    }

    #[test]
    fn test_materialize_non_monotonic_2() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();

        // +0 to -301 to -300 is ok, but materialization fails since we had 300 - 301 < 0!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.sub(301));
        assert_ok!(aggregator.add(1));
        assert_eq!(aggregator.value, 300);
        assert_eq!(aggregator.state, AggregatorState::NegativeDelta);
        assert_err!(aggregator.read_and_materialize(&resolver, &aggregator_v1_id_for_test(600)));
    }

    #[test]
    fn test_add_overflow() {
        let mut aggregator_data = AggregatorData::default();

        // +0 to +800 > 600!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_err!(aggregator.add(800));

        // 0 + 300 > 200!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(200), 200)
            .expect("Get aggregator failed");
        assert_err!(aggregator.add(300));
    }

    #[test]
    fn test_sub_underflow() {
        let mut aggregator_data = AggregatorData::default();
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200);

        // +0 to -601 is impossible!
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_err!(aggregator.sub(601));

        // Similarly, we cannot subtract anything from 0...
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(200), 200)
            .expect("Get aggregator failed");
        assert_err!(aggregator.sub(2));
    }

    #[test]
    fn test_commutative() {
        let mut aggregator_data = AggregatorData::default();

        // +200 -300 +50 +300 -25 +375 -600.
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");
        assert_ok!(aggregator.add(200));
        assert_ok!(aggregator.sub(300));

        assert_eq!(aggregator.value, 100);
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .max_achieved_positive_delta,
            200
        );
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .min_achieved_negative_delta,
            100
        );
        assert_eq!(aggregator.state, AggregatorState::NegativeDelta);

        assert_ok!(aggregator.add(50));
        assert_ok!(aggregator.add(300));
        assert_ok!(aggregator.sub(25));

        assert_eq!(aggregator.value, 225);
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .max_achieved_positive_delta,
            250
        );
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .min_achieved_negative_delta,
            100
        );
        assert_eq!(aggregator.state, AggregatorState::PositiveDelta);

        assert_ok!(aggregator.add(375));
        assert_ok!(aggregator.sub(600));

        assert_eq!(aggregator.value, 0);
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .max_achieved_positive_delta,
            600
        );
        assert_eq!(
            aggregator
                .history
                .as_ref()
                .unwrap()
                .min_achieved_negative_delta,
            100
        );
        assert_eq!(aggregator.state, AggregatorState::PositiveDelta);
    }

    #[test]
    #[should_panic]
    fn test_history_validation_in_data_state() {
        let mut aggregator_data = AggregatorData::default();

        // Validation panics if history is not set. This is an invariant
        // violation and should never happen.
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200);
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(200), 200)
            .expect("Getting an aggregator should succeed");
        aggregator
            .validate_history(0)
            .expect("Should not be called because validation panics");
    }

    #[test]
    fn test_history_validation_in_delta_state() {
        let mut aggregator_data = AggregatorData::default();

        // Some aggregator with a max_value of 100 in a delta state.
        let id = aggregator_v1_id_for_test(100);
        let aggregator = aggregator_data
            .get_aggregator(id, 100)
            .expect("Getting an aggregator should succeed");

        // Aggregator of +0 with minimum of -50 and maximum of +50.
        aggregator.add(50).unwrap();
        aggregator.sub(100).unwrap();
        aggregator.add(50).unwrap();

        // Valid history: 50+50-100+50.
        assert_ok!(aggregator.validate_history(50));

        // Underflow and overflow are unvalidated.
        assert_err!(aggregator.validate_history(49));
        assert_err!(aggregator.validate_history(51));
    }
}
