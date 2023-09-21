// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::{ok_overflow, BoundedMath, SignedU128},
    delta_math::DeltaHistory,
    resolver::{AggregatorReadMode, AggregatorResolver},
    types::{
        code_invariant_error, expect_ok, AggregatorID, AggregatorValue, AggregatorVersionedID,
        DelayedFieldsSpeculativeError, PanicError, PanicOr, PanicOrResult, ReadPosition,
        SnapshotToStringFormula, SnapshotValue,
    },
};
use aptos_types::{state_store::state_key::StateKey, vm_status::StatusCode};
use claims::assert_matches;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

/// Describes how the `speculative_start_value` in `AggregatorState` was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeculativeStartValue {
    // The speculative_start_value is not yet initialized
    Unset,
    // The speculative_start_value was obtained by reading
    // the last committed value of the aggregator from MVHashmap.
    // WARNING: any use of this value should be captured as a restriction
    // in the change set, as value received here is not track as part of the
    // read conflict!!
    // Only current restriction is DeltaHistory, and only correct usage is
    // that can be returned to the caller is via try_add/try_sub methods!
    LastCommittedValue(u128),
    // The speculative_start_value was obtained by performing a read
    // procedure on the aggregator, which involves aggregating deltas
    // present at the read time
    AggregatedValue(u128),
}

impl SpeculativeStartValue {
    // WARNING: any use of this value should be captured as a restriction
    // in the change set, as value received here is not track as part of the
    // read conflict!!
    // Only current restriction is DeltaHistory, and only correct usage is
    // that can be returned to the caller is via try_add/try_sub methods!
    pub fn get_any_value(&self) -> Result<u128, PanicError> {
        match self {
            SpeculativeStartValue::Unset => Err(code_invariant_error(
                "Tried calling get_any_value on Unset speculative value",
            )),
            SpeculativeStartValue::LastCommittedValue(value) => Ok(*value),
            SpeculativeStartValue::AggregatedValue(value) => Ok(*value),
        }
    }

    pub fn get_value_for_read(&self) -> Result<u128, PanicError> {
        match self {
            SpeculativeStartValue::Unset => Err(code_invariant_error(
                "Tried calling get_value_for_read on Unset speculative value",
            )),
            SpeculativeStartValue::LastCommittedValue(_) => Err(code_invariant_error(
                "Tried calling get_value_for_read on LastCommittedValue speculative value",
            )),
            SpeculativeStartValue::AggregatedValue(value) => Ok(*value),
        }
    }
}

/// Describes the state of each aggregator instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregatorState {
    // If aggregator is created in this transaction.
    Create {
        value: u128,
    },
    Delta {
        speculative_start_value: SpeculativeStartValue,
        delta: SignedU128,
        history: DeltaHistory,
    },
}

// Aggregator snapshot is immutable struct, once created - value is fixed.
// If we want to provide mutability APIs in the future, it should be
// copy-on-write - i.e. a new aggregator_id should be created for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregatorSnapshotState {
    // Created in this transaction, with explicit value
    Create {
        value: SnapshotValue,
    },
    // Created in this transaction, via snapshot(&aggregator)
    Delta {
        base_aggregator: AggregatorID,
        delta: SignedU128,
    },
    // Created in this transaction, via string_concat(prefix, &snapshot, suffix)
    Derived {
        base_snapshot: AggregatorID,
        formula: SnapshotToStringFormula,
    },
    // Accessed in this transaction, based on the ID
    Reference {
        // always expensive/aggregated read
        speculative_value: SnapshotValue,
    },
}

#[derive(Debug)]
pub struct AggregatorSnapshot {
    // The identifier used to identify the aggregator.
    #[allow(dead_code)]
    id: AggregatorID,

    state: AggregatorSnapshotState,
}

impl AggregatorSnapshot {
    pub fn into(self) -> AggregatorSnapshotState {
        self.state
    }
}

/// Internal aggregator data structure.
#[derive(Debug)]
pub struct Aggregator {
    // The identifier used to identify the aggregator.
    id: AggregatorVersionedID,
    // Describes an upper bound of an aggregator. If value of the aggregator
    // exceeds it, the aggregator overflows.
    pub max_value: u128,
    // Describes a state of an aggregator.
    pub state: AggregatorState,
}

fn get_aggregator_v2_value_from_storage(
    id: &AggregatorID,
    resolver: &dyn AggregatorResolver,
    mode: AggregatorReadMode,
) -> PanicOrResult<AggregatorValue, DelayedFieldsSpeculativeError> {
    // TODO transform unexpected errors into PanicError
    resolver
        .get_aggregator_v2_value(id, mode)
        .map_err(|_err| PanicOr::Or(DelayedFieldsSpeculativeError::NotFound(*id)))
}

impl Aggregator {
    #[cfg(test)]
    pub fn get_history(&self) -> Option<&DeltaHistory> {
        match &self.state {
            AggregatorState::Create { .. } => None,
            AggregatorState::Delta { history, .. } => Some(history),
        }
    }

    /// Returns error if transaction is in invalid state, and should be re-executed.
    /// Returns true if addition succeeded, and false if it would overflow.
    pub fn try_add(
        &mut self,
        input: u128,
        resolver: &dyn AggregatorResolver,
    ) -> PartialVMResult<bool> {
        if input > self.max_value {
            // We do not have to record the overflow.
            // We record the delta that result in overflows/underflows so that when we compute the actual value
            // of aggregator, we can figure out if the output of try_add/try_sub changes.
            // When input exceeds max_value, we know that no matter what the starting value of the
            // aggregator is, it always results in an overflow.
            return Ok(false);
        }
        let math = BoundedMath::new(self.max_value);
        self.read_last_committed_aggregator_value(resolver)?;
        match &mut self.state {
            AggregatorState::Create { value } => {
                // If aggregator knows the value, add directly and keep the state.
                match math.unsigned_add(*value, input) {
                    Ok(new_value) => {
                        *value = new_value;
                        Ok(true)
                    },
                    Err(_) => Ok(false),
                }
            },
            AggregatorState::Delta {
                speculative_start_value,
                delta,
                history,
            } => {
                let cur_value = expect_ok(
                    math.unsigned_add_delta(speculative_start_value.get_any_value()?, delta),
                )?;

                if math.unsigned_add(cur_value, input).is_err() {
                    let overflow_delta =
                        expect_ok(ok_overflow(math.unsigned_add_delta(input, delta)))?;

                    // if value overflowed, we don't need to record it
                    if let Some(overflow_delta) = overflow_delta {
                        history.record_overflow(overflow_delta);
                    }
                    Ok(false)
                } else {
                    let new_delta =
                        expect_ok(math.signed_add(delta, &SignedU128::Positive(input)))?;
                    *delta = new_delta;
                    history.record_success(new_delta);
                    Ok(true)
                }
            },
        }
    }

    /// Returns error if transaction is in invalid state, and should be re-executed.
    /// Returns true if subtraction succeeded, and false if it would underflow.
    pub fn try_sub(
        &mut self,
        input: u128,
        resolver: &dyn AggregatorResolver,
    ) -> PartialVMResult<bool> {
        if input > self.max_value {
            // We do not have to record the underflow.
            // We record the delta that result in overflows/underflows so that when we compute the actual value
            // of aggregator, we can figure out if the output of try_add/try_sub changes.
            // When input exceeds max_value, we know that no matter what the starting value of the
            // aggregator is, it always results in an underflow.
            return Ok(false);
        }
        let math = BoundedMath::new(self.max_value);
        self.read_last_committed_aggregator_value(resolver)?;
        match &mut self.state {
            AggregatorState::Create { value } => {
                // If aggregator knows the value, add directly and keep the state.
                match math.unsigned_subtract(*value, input) {
                    Ok(new_value) => {
                        *value = new_value;
                        Ok(true)
                    },
                    Err(_) => Ok(false),
                }
            },
            AggregatorState::Delta {
                speculative_start_value,
                delta,
                history,
            } => {
                let cur_value = expect_ok(
                    math.unsigned_add_delta(speculative_start_value.get_any_value()?, delta),
                )?;

                if cur_value < input {
                    let underflow_delta =
                        expect_ok(ok_overflow(math.unsigned_add_delta(input, &delta.minus())))?;
                    // If value overflowed (delta was smaller than -max_value), we don't need to record it.
                    if let Some(underflow_delta) = underflow_delta {
                        history.record_underflow(underflow_delta);
                    }
                    Ok(false)
                } else {
                    let new_delta =
                        expect_ok(math.signed_add(delta, &SignedU128::Negative(input)))?;
                    *delta = new_delta;
                    history.record_success(new_delta);
                    Ok(true)
                }
            },
        }
    }

    fn get_aggregator_value_from_storage(
        id: &AggregatorVersionedID,
        resolver: &dyn AggregatorResolver,
        mode: AggregatorReadMode,
    ) -> PartialVMResult<u128> {
        match id {
            AggregatorVersionedID::V1(state_key) => resolver
                .get_aggregator_v1_value(state_key, mode)
                .map_err(|e| {
                    extension_error(format!("Could not find the value of the aggregator: {}", e))
                })?
                .ok_or_else(|| {
                    extension_error(format!(
                        "Could not read from deleted aggregator at {:?}",
                        id
                    ))
                }),
            AggregatorVersionedID::V2(id) => {
                let value = get_aggregator_v2_value_from_storage(id, resolver, mode)?;
                Ok(value.into_aggregator_value()?)
            },
        }
    }

    /// Implements logic for doing a "cheap read" of an aggregator.
    /// Reads the last committed value of the aggregator that's known at the
    /// time of the call, and as such, can be computed efficiently (i.e. no
    /// need to consider any speculative state, deltas, etc)
    /// This method has a sideffect, of updating `speculative_start_value` with
    /// `LastCommittedValue` variant.
    /// `get_any_value()` is guaranteed to succeed after this call.
    /// This needs to be called before updating aggregator with delta's, i.e. if
    /// aggregator is in Delta state, delta should be 0, and history should be empty.
    pub fn read_last_committed_aggregator_value(
        &mut self,
        resolver: &dyn AggregatorResolver,
    ) -> PartialVMResult<()> {
        if let AggregatorState::Delta {
            speculative_start_value,
            delta,
            history,
        } = &mut self.state
        {
            // If value is Unset, we read it
            if let SpeculativeStartValue::Unset = speculative_start_value {
                if !delta.is_zero() || !history.is_empty() {
                    Err(code_invariant_error(
                        "Delta or history not empty with Unset speculative value",
                    ))?;
                }

                let value_from_storage = Self::get_aggregator_value_from_storage(
                    &self.id,
                    resolver,
                    AggregatorReadMode::LastCommitted,
                )?;

                *speculative_start_value =
                    SpeculativeStartValue::LastCommittedValue(value_from_storage)
            }
        }
        Ok(())
    }

    /// Implements logic for doing an "expensive read" of an aggregator.
    /// This means that we perform a full read of an aggregator, that may involve
    /// aggregating any speculative delta operations and can thus be more expensive
    /// than reading the latest committed value.
    /// This method has a sideffect, of updating `speculative_start_value` with
    /// `LastCommittedValue` variant.
    /// Both `get_any_value()` and `get_value_for_read()` are guaranteed to succeed
    /// after this call.
    pub fn read_aggregated_aggregator_value(
        &mut self,
        resolver: &dyn AggregatorResolver,
        read_position: ReadPosition,
    ) -> PartialVMResult<u128> {
        match &mut self.state {
            AggregatorState::Create { value } => {
                match read_position {
                    ReadPosition::BeforeCurrentTxn => {
                        Err(code_invariant_error(
                            "Asking for aggregator value BeforeCurrentTxn that was created in this transaction",
                        ).into())
                    },
                    ReadPosition::AfterCurrentTxn => {
                        // If aggregator knows the value, return it.
                        Ok(*value)
                    },
                }
            },
            AggregatorState::Delta {
                speculative_start_value,
                delta,
                history,
            } => {
                let math = BoundedMath::new(self.max_value);
                // If we performed an "expensive read" operation before, use it.
                if let SpeculativeStartValue::AggregatedValue(start_value) = speculative_start_value
                {
                    match read_position {
                        ReadPosition::BeforeCurrentTxn => {
                            return Ok(*start_value);
                        },
                        ReadPosition::AfterCurrentTxn => {
                            // state should always be valid, so this should never fail
                            return Ok(expect_ok(math.unsigned_add_delta(*start_value, delta))?);
                        },
                    }
                }
                if let SpeculativeStartValue::Unset = speculative_start_value {
                    if !delta.is_zero() || !history.is_empty() {
                        Err(code_invariant_error(
                            "Delta or history not empty with Unset speculative value",
                        ))?;
                    }
                }

                // Otherwise, we have to go to storage and read the value.
                let value_from_storage = Self::get_aggregator_value_from_storage(
                    &self.id,
                    resolver,
                    AggregatorReadMode::Aggregated,
                )?;

                // Validate history.
                history.validate_against_base_value(value_from_storage, self.max_value)?;
                // Applyng shouldn't fail after validation
                let result = expect_ok(math.unsigned_add_delta(value_from_storage, delta))?;

                *speculative_start_value =
                    SpeculativeStartValue::AggregatedValue(value_from_storage);

                match read_position {
                    ReadPosition::BeforeCurrentTxn => Ok(value_from_storage),
                    ReadPosition::AfterCurrentTxn => Ok(result),
                }
            },
        }
    }

    /// Unpacks aggregator into its fields.
    pub fn into(self) -> (u128, AggregatorState) {
        (self.max_value, self.state)
    }
}

/// Stores all information about aggregators (how many have been created or
/// removed), what are their states, etc. per single transaction).
#[derive(Default)]
pub struct AggregatorData {
    // All aggregators that were created in the current transaction, stored as ids.
    // Used to filter out aggregators that were created and destroyed in the
    // within a single transaction.
    new_aggregators: BTreeSet<AggregatorVersionedID>,
    // All aggregators that were destroyed in the current transaction, stored as ids.
    destroyed_aggregators: BTreeSet<StateKey>,
    // All aggregator instances that exist in the current transaction.
    aggregators: BTreeMap<AggregatorVersionedID, Aggregator>,
    // All aggregator snapshot instances that exist in the current transaction.
    aggregator_snapshots: BTreeMap<AggregatorID, AggregatorSnapshot>,
}

impl AggregatorData {
    /// Returns a mutable reference to an aggregator with `id` and a `max_value`.
    /// If transaction that is currently executing did not initialize it,
    /// a new aggregator instance is created.
    /// Note: when we say "aggregator instance" here we refer to Rust struct and
    /// not to the Move aggregator.
    pub fn get_aggregator(
        &mut self,
        id: AggregatorVersionedID,
        max_value: u128,
    ) -> PartialVMResult<&mut Aggregator> {
        let aggregator = self
            .aggregators
            .entry(id.clone())
            .or_insert_with(|| Aggregator {
                id: id.clone(),
                state: AggregatorState::Delta {
                    speculative_start_value: SpeculativeStartValue::Unset,
                    delta: SignedU128::Positive(0),
                    history: DeltaHistory::new(),
                },
                max_value,
            });
        if aggregator.max_value != max_value {
            Err(code_invariant_error(format!(
                "Max value for the aggregator changed ({} -> {})",
                aggregator.max_value, max_value
            )))?;
        }
        Ok(aggregator)
    }

    /// Returns the number of aggregators that are used in the current transaction.
    pub fn num_aggregators(&self) -> u128 {
        self.aggregators.len() as u128
    }

    /// Creates and a new Aggregator with a given `id` and a `max_value`. The value
    /// of a new aggregator is always known, therefore it is created in a data
    /// state, with a zero-initialized value.
    pub fn create_new_aggregator(&mut self, id: AggregatorVersionedID, max_value: u128) {
        let aggregator = Aggregator {
            id: id.clone(),
            state: AggregatorState::Create { value: 0 },
            max_value,
        };
        self.aggregators.insert(id.clone(), aggregator);
        self.new_aggregators.insert(id);
    }

    /// If aggregator has been used in this transaction, it is removed. Otherwise,
    /// it is marked for deletion.
    pub fn remove_aggregator_v1(&mut self, id: AggregatorVersionedID) {
        // Only V1 aggregators can be removed.
        assert_matches!(id, AggregatorVersionedID::V1(_));

        self.aggregators.remove(&id);

        if self.new_aggregators.contains(&id) {
            self.new_aggregators.remove(&id);
        } else {
            // This avoids cloning the state key.
            let state_key = id.try_into().expect("V1 identifiers are state keys");
            self.destroyed_aggregators.insert(state_key);
        }
    }

    pub fn snapshot(
        &mut self,
        aggregator_id: AggregatorID,
        aggregator_max_value: u128,
        resolver: &dyn AggregatorResolver,
    ) -> PartialVMResult<AggregatorID> {
        let aggregator = self.get_aggregator(
            AggregatorVersionedID::V2(aggregator_id),
            aggregator_max_value,
        )?;

        let snapshot_state = match aggregator.state {
            // If aggregator is in Create state, we don't need to depend on it, and can just take the value.
            AggregatorState::Create { value } => AggregatorSnapshotState::Create {
                value: SnapshotValue::Integer(value),
            },
            AggregatorState::Delta { delta, .. } => AggregatorSnapshotState::Delta {
                base_aggregator: aggregator_id,
                delta,
            },
        };

        let snapshot_id = resolver.generate_aggregator_v2_id();
        self.aggregator_snapshots
            .insert(snapshot_id, AggregatorSnapshot {
                id: snapshot_id,
                state: snapshot_state,
            });
        Ok(snapshot_id)
    }

    pub fn create_new_snapshot(
        &mut self,
        value: SnapshotValue,
        resolver: &dyn AggregatorResolver,
    ) -> AggregatorID {
        let snapshot_state = AggregatorSnapshotState::Create { value };
        let snapshot_id = resolver.generate_aggregator_v2_id();

        self.aggregator_snapshots
            .insert(snapshot_id, AggregatorSnapshot {
                id: snapshot_id,
                state: snapshot_state,
            });
        snapshot_id
    }

    pub fn read_snapshot(
        &mut self,
        snapshot_id: AggregatorID,
        resolver: &dyn AggregatorResolver,
    ) -> PartialVMResult<SnapshotValue> {
        // Since we need the value - if it is not present, we need to do the "aggregated read" to get it.
        // need to clone here, so we can call self.read_snapshot below.
        let snapshot_state = match self.aggregator_snapshots.entry(snapshot_id) {
            Entry::Vacant(entry) => {
                let value_from_storage = get_aggregator_v2_value_from_storage(
                    &snapshot_id,
                    resolver,
                    AggregatorReadMode::Aggregated,
                )?;
                entry
                    .insert(AggregatorSnapshot {
                        id: snapshot_id,
                        state: AggregatorSnapshotState::Reference {
                            speculative_value: SnapshotValue::try_from(value_from_storage)?,
                        },
                    })
                    .state
                    .clone()
            },
            Entry::Occupied(entry) => entry.get().state.clone(),
        };
        match snapshot_state {
            AggregatorSnapshotState::Create { value } => Ok(value),
            AggregatorSnapshotState::Delta {
                base_aggregator,
                delta,
            } => match self
                .aggregators
                .get_mut(&AggregatorVersionedID::V2(base_aggregator))
            {
                Some(aggregator) => {
                    // We need to make sure speculative_start_value is in a state for read,
                    // but we need the value at the start of the transaction
                    let speculative_start_value = aggregator.read_aggregated_aggregator_value(
                        resolver,
                        ReadPosition::BeforeCurrentTxn,
                    )?;
                    Ok(SnapshotValue::Integer(expect_ok(
                        BoundedMath::new(aggregator.max_value)
                            .unsigned_add_delta(speculative_start_value, &delta),
                    )?))
                },
                None => Err(PartialVMError::from(code_invariant_error(
                    "AggregatorSnapshotState::Delta without base aggregator being set",
                ))),
            },
            AggregatorSnapshotState::Derived {
                base_snapshot,
                formula,
            } => {
                let base = self.read_snapshot(base_snapshot, resolver)?;
                match base {
                    SnapshotValue::Integer(v) => Ok(SnapshotValue::String(formula.apply_to(v))),
                    SnapshotValue::String(_) => Err(PartialVMError::from(code_invariant_error(
                        "Tried calling concat on String SnapshotValue",
                    ))),
                }
            },
            AggregatorSnapshotState::Reference { speculative_value } => Ok(speculative_value),
        }
    }

    pub fn string_concat(
        &mut self,
        id: AggregatorID,
        prefix: Vec<u8>,
        suffix: Vec<u8>,
        resolver: &dyn AggregatorResolver,
    ) -> AggregatorID {
        let new_id = resolver.generate_aggregator_v2_id();

        let snapshot_state = AggregatorSnapshotState::Derived {
            base_snapshot: id,
            formula: SnapshotToStringFormula::Concat { prefix, suffix },
        };

        self.aggregator_snapshots
            .insert(new_id, AggregatorSnapshot {
                id: new_id,
                state: snapshot_state,
            });
        new_id
    }

    /// Unpacks aggregator data.
    pub fn into(
        self,
    ) -> (
        BTreeSet<AggregatorVersionedID>,
        BTreeSet<StateKey>,
        BTreeMap<AggregatorVersionedID, Aggregator>,
        BTreeMap<AggregatorID, AggregatorSnapshot>,
    ) {
        (
            self.new_aggregators,
            self.destroyed_aggregators,
            self.aggregators,
            self.aggregator_snapshots,
        )
    }
}

/// Returns partial VM error on extension failure.
pub fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::{aggregator_v1_id_for_test, aggregator_v1_state_key_for_test, FakeAggregatorView};
    use claims::{assert_err, assert_ok, assert_ok_eq, assert_some_eq};

    #[test]
    fn test_aggregator_not_in_storage() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();
        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(300), 700)
            .unwrap();
        assert_err!(aggregator.read_last_committed_aggregator_value(&resolver));
        assert_err!(
            aggregator.read_aggregated_aggregator_value(&resolver, ReadPosition::AfterCurrentTxn)
        );
        assert_err!(aggregator.try_add(100, &resolver));
        assert_err!(aggregator.try_sub(1, &resolver));
    }

    #[test]
    fn test_operations_on_new_aggregator() {
        let resolver = FakeAggregatorView::default();
        let mut aggregator_data = AggregatorData::default();
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(200), 200)
            .expect("Get aggregator failed");

        assert_eq!(aggregator.state, AggregatorState::Create { value: 0 });
        assert_ok!(aggregator.try_add(100, &resolver));
        assert_eq!(aggregator.state, AggregatorState::Create { value: 100 });
        assert!(aggregator.try_sub(50, &resolver).unwrap());
        assert_eq!(aggregator.state, AggregatorState::Create { value: 50 });
        assert!(!aggregator.try_sub(70, &resolver).unwrap());
        assert_eq!(aggregator.state, AggregatorState::Create { value: 50 });
        assert!(!aggregator.try_add(170, &resolver).unwrap());
        assert_eq!(aggregator.state, AggregatorState::Create { value: 50 });
        assert_ok_eq!(
            aggregator.read_aggregated_aggregator_value(&resolver, ReadPosition::AfterCurrentTxn),
            50
        );
    }
    #[test]
    fn test_successful_operations_in_delta_mode() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 100);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::Unset,
            delta: SignedU128::Positive(0),
            history: DeltaHistory {
                max_achieved_positive_delta: 0,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
        assert_ok!(aggregator.try_add(400, &sample_resolver));
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(100),
            delta: SignedU128::Positive(400),
            history: DeltaHistory {
                max_achieved_positive_delta: 400,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
        assert_ok!(aggregator.try_sub(470, &sample_resolver));
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(100),
            delta: SignedU128::Negative(70),
            history: DeltaHistory {
                max_achieved_positive_delta: 400,
                min_achieved_negative_delta: 70,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
        assert_ok_eq!(
            aggregator
                .read_aggregated_aggregator_value(&sample_resolver, ReadPosition::AfterCurrentTxn),
            30
        );
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::AggregatedValue(100),
            delta: SignedU128::Negative(70),
            history: DeltaHistory {
                max_achieved_positive_delta: 400,
                min_achieved_negative_delta: 70,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
    }

    #[test]
    fn test_history_updates() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 100);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::Unset,
            delta: SignedU128::Positive(0),
            history: DeltaHistory {
                max_achieved_positive_delta: 0,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
        assert_ok!(aggregator.try_add(300, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_ok!(aggregator.try_add(100, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_ok!(aggregator.try_sub(450, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_ok!(aggregator.try_add(200, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_ok!(aggregator.try_add(350, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 500,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_ok!(aggregator.try_sub(600, &sample_resolver));
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 500,
            min_achieved_negative_delta: 100,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
    }

    #[test]
    fn test_aggregator_overflows() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 100);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert!(aggregator.try_add(400, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert!(aggregator.try_sub(450, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_add(601, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_add(575, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: Some(525),
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_add(551, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: Some(501),
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_add(570, &sample_resolver).unwrap());
        assert_some_eq!(aggregator.get_history(), &DeltaHistory {
            max_achieved_positive_delta: 400,
            min_achieved_negative_delta: 50,
            min_overflow_positive_delta: Some(501),
            max_underflow_negative_delta: None,
        });
    }

    fn assert_delta_state(
        aggregator: &AggregatorState,
        speculative_start_value: u128,
        delta: i128,
        history: DeltaHistory,
    ) {
        assert_eq!(aggregator, &AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(
                speculative_start_value
            ),
            delta: if delta > 0 {
                SignedU128::Positive(delta as u128)
            } else {
                SignedU128::Negative((-delta) as u128)
            },
            history,
        });
    }

    #[test]
    fn test_aggregator_underflows() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert!(aggregator.try_add(300, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_sub(650, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert!(!aggregator.try_sub(550, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: Some(250),
        });
        assert!(!aggregator.try_sub(525, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: Some(225),
        });
        assert!(!aggregator.try_sub(540, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: Some(225),
        });
        assert!(!aggregator.try_sub(501, &sample_resolver).unwrap());
        assert_delta_state(&aggregator.state, 200, 300, DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: Some(201),
        });
    }

    #[test]
    fn test_change_in_base_value_1() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert_ok!(aggregator.try_add(300, &sample_resolver));
        assert_ok!(aggregator.try_sub(400, &sample_resolver));
        assert_ok!(aggregator.try_add(400, &sample_resolver));
        assert_ok!(aggregator.try_sub(500, &sample_resolver));
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(200),
            delta: SignedU128::Negative(200),
            history: DeltaHistory {
                max_achieved_positive_delta: 300,
                min_achieved_negative_delta: 200,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }
        });
        if let AggregatorState::Delta { history, .. } = aggregator.state {
            assert_ok!(history.validate_against_base_value(200, aggregator.max_value,));
            assert_err!(history.validate_against_base_value(199, aggregator.max_value,));
            assert_ok!(history.validate_against_base_value(300, aggregator.max_value,));
            assert_err!(history.validate_against_base_value(301, aggregator.max_value,));
        }
    }

    #[test]
    fn test_change_in_base_value_2() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert!(!aggregator.try_add(401, &sample_resolver).unwrap());
        assert!(aggregator.try_add(300, &sample_resolver).unwrap());
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(200),
            delta: SignedU128::Positive(300),
            history: DeltaHistory {
                max_achieved_positive_delta: 300,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: Some(401),
                max_underflow_negative_delta: None,
            }
        });

        if let AggregatorState::Delta { history, .. } = aggregator.state {
            assert_err!(history.validate_against_base_value(199, aggregator.max_value,));
            assert_ok!(history.validate_against_base_value(200, aggregator.max_value,));
            assert_ok!(history.validate_against_base_value(300, aggregator.max_value,));
            assert_err!(history.validate_against_base_value(301, aggregator.max_value,));
        }
    }

    #[test]
    fn test_change_in_base_value_3() {
        let mut aggregator_data = AggregatorData::default();
        let mut sample_resolver = FakeAggregatorView::default();
        sample_resolver.set_from_state_key(aggregator_v1_state_key_for_test(600), 200);

        let aggregator = aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .expect("Get aggregator failed");

        assert!(aggregator.try_sub(100, &sample_resolver).unwrap());
        assert!(!aggregator.try_sub(101, &sample_resolver).unwrap());
        assert!(aggregator.try_add(300, &sample_resolver).unwrap());
        assert_eq!(aggregator.state, AggregatorState::Delta {
            speculative_start_value: SpeculativeStartValue::LastCommittedValue(200),
            delta: SignedU128::Positive(200),
            history: DeltaHistory {
                max_achieved_positive_delta: 200,
                min_achieved_negative_delta: 100,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: Some(201),
            }
        });

        if let AggregatorState::Delta { history, .. } = aggregator.state {
            assert_ok!(history.validate_against_base_value(100, aggregator.max_value,));
            assert_ok!(history.validate_against_base_value(199, aggregator.max_value,));
            assert_ok!(history.validate_against_base_value(200, aggregator.max_value,));
            assert_err!(history.validate_against_base_value(201, aggregator.max_value,));
            assert_err!(history.validate_against_base_value(400, aggregator.max_value,));
        }
    }
}
