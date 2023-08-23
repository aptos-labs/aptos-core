use aptos_state_view::StateView;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp, validator_config};
use move_binary_format::errors::{Location, PartialVMResult};
use move_core_types::vm_status::{VMStatus, StatusCode};

use crate::{aggregator_extension::{AggregatorState, validate_history, DeltaValue}, delta_change_set::{deserialize, serialize, addition, subtraction}, module::AGGREGATOR_MODULE};


/// Represents a single aggregator change.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AggregatorChange {
    // The state of the aggregator
    State {
        max_value: u128,
        state: AggregatorState,
    },
    // A value should be deleted from the storage.
    Delete,
}

impl AggregatorChange {
    /// Tries to materialize the AggregatorChange with a given state key. 
    /// If materialization succeeds, a write op is produced. Otherwise, an
    /// error VM status is returned.
    // TODO: This function currently works only for the Aggregator V1.
    pub fn try_into_write_op(
        self,
        state_view: &dyn StateView,
        state_key: &StateKey,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        match self {
            AggregatorChange::State { .. } => {
                // In case storage fails to fetch the value, return immediately.
                    let maybe_value = state_view
                    .get_state_value_bytes(state_key)
                    .map_err(|e| VMStatus::error(StatusCode::STORAGE_ERROR, Some(e.to_string())))?;

                // Otherwise we have to apply the current aggregator change to the storage value.
                match maybe_value {
                    Some(bytes) => {
                        let base = deserialize(&bytes);
                        self.apply_aggregator_change_to(base)
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
            },
            AggregatorChange::Delete => Ok(WriteOp::Deletion),
        }
    }

    /// Returns the result of applying the AggregatorState to the base value
    /// Returns error if postcondition is not satisfied.
    pub fn apply_aggregator_change_to(&self, base: u128) -> PartialVMResult<u128> {
        if let AggregatorChange::State { max_value, state } = self {
            match state {
                AggregatorState::Data { value } => {
                    Ok(*value)
                },
                AggregatorState::Delta { delta, .. } => {
                    // First, validate if the current delta operation can be applied to the base.
                    validate_history(base, *max_value, state)?;
                    match delta {
                        DeltaValue::Positive(value) => addition(base, *value, *max_value),
                        DeltaValue::Negative(value) => subtraction(base, *value),
                    }
                }
            }
        } else {
            unreachable!("Cannot apply deletion aggregator change to a base");
        }
    }

    /// Applies self on top of previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows
    /// correctly.
    pub fn merge_with_previous_aggregator_change(&mut self, previous_change: AggregatorChange) -> PartialVMResult<()> {
        match self {
            AggregatorChange::State { max_value, state } => {
                match previous_change {
                    AggregatorChange::State { max_value: prev_max_value, state: prev_state } => {
                        assert_eq!(
                            *max_value, prev_max_value,
                            "Cannot merge aggregator changes with different max_values",
                        );
                        match state {
                            AggregatorState::Data { value } => {
                                // If the current state is Data, then merging with previous state won't change anything.
                                return Ok(());
                            },
                            AggregatorState::Delta { speculative_source, speculative_start_value, delta, history } => {
                                match prev_state {
                                    AggregatorState::Data { value } => {
                                        // When prev_state is Data { value }, and current state is Delta { delta }, merging them into Data { value + delta }
                                        validate_history(value, *max_value, state)?;
                                        *state = AggregatorState::Data {
                                                value: match delta {
                                                    DeltaValue::Positive(current_delta) => addition(value, *current_delta, *max_value)?,
                                                    DeltaValue::Negative(current_delta) => subtraction(value, *current_delta)?,
                                                }
                                            };
                                    },
                                    AggregatorState::Delta { speculative_start_value: prev_speculative_start_value, speculative_source: prev_speculative_source, delta: prev_delta, history: prev_history } => {
                                        // Merge the history of the previous delta with the current delta.
                                        *speculative_start_value = match prev_delta {
                                            DeltaValue::Positive(prev_delta) => addition(*speculative_start_value, prev_delta, *max_value)?,
                                            DeltaValue::Negative(prev_delta) => subtraction(*speculative_start_value, prev_delta)?,
                                        };

                                        // Useful macro for merging deltas of the same sign, e.g. +A+B or -A-B.
                                        // In this cases we compute the absolute sum of deltas (A+B) and use plus
                                        // or minus sign accordingly.
                                        macro_rules! update_same_sign {
                                            ($sign:ident, $a:ident, $b:ident) => {
                                                delta = $sign(addition($a, $b, max_value)?)
                                            };
                                        }

                                        // Another useful macro, this time for merging deltas with different signs, such
                                        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
                                        // and possibly flip a sign.
                                        macro_rules! update_different_sign {
                                            ($a:ident, $b:ident) => {
                                                if $a >= $b {
                                                    *delta = DeltaValue::Positive(subtraction($a, $b)?);
                                                } else {
                                                    *delta = DeltaValue::Negative(subtraction($b, $a)?);
                                                }
                                            };
                                        }
                                        // History check passed, and we are ready to update the actual values now.
                                        match prev_delta {
                                            DeltaValue::Positive(prev_value) => match delta {
                                                DeltaValue::Positive(self_value) => update_same_sign!(DeltaValue::Positive, prev_value, *self_value),
                                                DeltaValue::Negative(self_value) => update_different_sign!(prev_value, *self_value)
                                            },
                                            DeltaValue::Negative(prev_value) => match delta {
                                                DeltaValue::Positive(self_value) => update_different_sign!(*self_value, prev_value),
                                                DeltaValue::Negative(self_value) => update_same_sign!(DeltaValue::Negative, prev_value, *self_value),
                                            },
                                        }
                                        
                                        // First, update the history values of this delta given that it starts from
                                        // +value or -value instead of 0. We should do this check to avoid cases like this:
                                        //
                                        // Suppose we have deltas with max_value of 100, and we have some `d2` which is +3 but it
                                        // was +99 at some point. Now, if we merge some `d1` which is +2 with `d2`, we get
                                        // the result is +5. However, it should not have happened because `d2` should hit
                                        // +2+99 > 100 at some point in history and fail.
                                        let shifted_max_achieved_positive_delta =
                                        self.shifted_max_achieved_positive_delta_by(&previous_delta)?;
                                        let shifted_min_achieved_negative_delta =
                                            self.shifted_min_achieved_negative_delta_by(&previous_delta)?;
                                        // Deltas have been merged successfully - update the history as well.
                                        self.history.max_achieved_positive_delta = u128::max(
                                            previous_delta.history.max_achieved_positive_delta,
                                            shifted_max_achieved_positive_delta,
                                        );
                                        self.history.min_achieved_negative_delta = u128::max(
                                            previous_delta.history.min_achieved_negative_delta,
                                            shifted_min_achieved_negative_delta,
                                        );
                                    }
                                }
                            }
                        }
                        Ok(())
                    },
                    AggregatorChange::Delete => {
                        // If the aggregator is deleted in the previously, then replace the current aggregator change with Delete.
                        *self = AggregatorChange::Delete;
                        Ok(())
                    }
                }
            },
            AggregatorChange::Delete => {
                // If the aggregator change is Delete, then merging with previous aggregator change won't change anything.
                Ok(())
            }
        }
    }


    /// Applies next aggregator change on top of self, merging two changes together. This is a reverse
    /// of `merge_with_previous_aggregator_change`.
    pub fn merge_with_next_aggregator_change(&mut self, next_change: AggregatorChange) -> PartialVMResult<()> {
        // Now self follows the other delta.
        let mut prev_change = next_change;
        std::mem::swap(self, &mut prev_change);

        // Perform the merge.
        self.merge_with_previous_aggregator_change(prev_change)?;
        Ok(())
    }
}
