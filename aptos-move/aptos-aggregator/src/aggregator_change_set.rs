// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_extension::{
        validate_history, AggregatorID, AggregatorState, DeltaHistory, SpeculativeStartValue,
    },
    bounded_math::{abort_error, addition_deltavalue, ok_overflow, ok_underflow},
};
use aptos_state_view::StateView;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::{StatusCode, VMStatus};

/// When merging aggregator changes of two transactions,
/// unable to merge the histories of the transactions.
pub(crate) const EMERGE_HISTORIES: u64 = 0x02_0008;

/// Represents a single aggregator change.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AggregatorChange {
    pub max_value: u128,
    pub state: AggregatorState,
    // `base_aggregator` is None for Aggregators.
    // `base_aggregator` is Some(id) for AggregatorSnapshots.
    pub base_aggregator: Option<AggregatorID>,
}

impl AggregatorChange {
    /// TODO: Implement this.
    pub fn materialize_aggregator(
        self,
        _state_view: &dyn StateView,
        _aggregator_id: &AggregatorID,
    ) -> anyhow::Result<AggregatorChange, VMStatus> {
        Ok(self)
    }

    /// Returns the result of applying the AggregatorState to the base value
    /// Returns error if postcondition is not satisfied.
    pub fn apply_aggregator_change_to(&self, base: u128) -> PartialVMResult<u128> {
        match self.state {
            AggregatorState::Data { value } => Ok(value),
            AggregatorState::Delta { delta, .. } => {
                // First, validate if the current delta operation can be applied to the base.
                validate_history(base, self.max_value, &self.state)?;
                Ok(addition_deltavalue(base, delta, self.max_value)?)
            },
        }
    }

    /// Applies self on top of previous change, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows correctly.
    /// TODO: What happens if the base aggregator is not none?
    pub fn merge_with_previous_aggregator_change(
        &mut self,
        previous_change: AggregatorChange,
    ) -> PartialVMResult<()> {
        assert_eq!(
            self.max_value, previous_change.max_value,
            "Cannot merge aggregator changes with different max_values",
        );
        // When the previous aggregator change is a snapshot, we cannot merge it with the current.
        if previous_change.base_aggregator.is_some() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Cannot merge aggregator snapshots".to_string()),
            );
        }
        match self.state {
            AggregatorState::Data { .. } => {
                // If the current state is Data, then merging with previous state won't change anything.
                return Ok(());
            },
            AggregatorState::Delta { delta, history, .. } => {
                match previous_change.state {
                    AggregatorState::Data { value } => {
                        // When prev_state is Data { value }, and current state is Delta { delta }, merging them into Data { value + delta }
                        validate_history(value, self.max_value, &self.state)?;
                        self.state = AggregatorState::Data {
                            value: addition_deltavalue(value, delta, self.max_value)?,
                        };
                    },
                    AggregatorState::Delta {
                        speculative_start_value: _,
                        delta: prev_delta,
                        history: prev_history,
                    } => {
                        let new_delta = prev_delta.add(&delta, self.max_value)?;

                        let new_min_overflow = {
                            let adjusted_min_overflow_positive_delta =
                                history.min_overflow_positive_delta.map_or(
                                    Ok(None),
                                    // Return Result<Option<u128>>. we want to have None on overflow,
                                    // and to fail the merging on underflow
                                    |min_overflow_positive_delta| {
                                        ok_overflow(addition_deltavalue(
                                            min_overflow_positive_delta,
                                            prev_delta,
                                            self.max_value,
                                        ))
                                    },
                                )?;

                            match (
                                adjusted_min_overflow_positive_delta,
                                prev_history.min_overflow_positive_delta,
                            ) {
                                (Some(a), Some(b)) => Some(u128::min(a, b)),
                                (a, b) => a.or(b),
                            }
                        };

                        let new_max_underflow = {
                            let adjusted_max_underflow_negative_delta =
                                history.max_underflow_negative_delta.map_or(
                                    Ok(None),
                                    // Return Result<Option<u128>>. we want to have None on overflow,
                                    // and to fail the merging on underflow
                                    |max_underflow_negative_delta| {
                                        ok_overflow(addition_deltavalue(
                                            max_underflow_negative_delta,
                                            prev_delta.minus(),
                                            self.max_value,
                                        ))
                                    },
                                )?;

                            match (
                                adjusted_max_underflow_negative_delta,
                                prev_history.max_underflow_negative_delta,
                            ) {
                                (Some(a), Some(b)) => Some(u128::min(a, b)),
                                (a, b) => a.or(b),
                            }
                        };

                        // new_max_achieved = max(prev_max_achieved, max_achieved + prev_delta)
                        // When adjusting max_achieved, if underflow - than the other is bigger,
                        // but if overflow - we fail the merge, as we cannot successfully achieve delta larger than max_value.
                        let new_max_achieved = ok_underflow(addition_deltavalue(
                            history.max_achieved_positive_delta,
                            prev_delta,
                            self.max_value,
                        ))?
                        .map_or(prev_history.max_achieved_positive_delta, |value| {
                            u128::max(prev_history.max_achieved_positive_delta, value)
                        });

                        // new_min_achieved = max(prev_min_achieved, min_achieved - prev_delta)
                        let new_min_achieved = ok_underflow(addition_deltavalue(
                            history.min_achieved_negative_delta,
                            prev_delta.minus(),
                            self.max_value,
                        ))?
                        .map_or(prev_history.min_achieved_negative_delta, |value| {
                            u128::max(prev_history.min_achieved_negative_delta, value)
                        });

                        if (new_min_overflow.is_some()
                            && new_min_overflow.unwrap() <= new_max_achieved)
                            || (new_max_underflow.is_some()
                                && new_max_underflow.unwrap() <= new_min_achieved)
                        {
                            return Err(abort_error(
                                "Unable to merge aggregator change histories",
                                EMERGE_HISTORIES,
                            ));
                        };

                        self.state = AggregatorState::Delta {
                            speculative_start_value: SpeculativeStartValue::Unset,
                            delta: new_delta,
                            history: DeltaHistory {
                                max_achieved_positive_delta: new_max_achieved,
                                min_achieved_negative_delta: new_min_achieved,
                                min_overflow_positive_delta: new_min_overflow,
                                max_underflow_negative_delta: new_max_underflow,
                            },
                        };
                    },
                }
            },
        }
        Ok(())
    }

    /// Applies next aggregator change on top of self, merging two changes together. This is a reverse
    /// of `merge_with_previous_aggregator_change`.
    pub fn merge_with_next_aggregator_change(
        &mut self,
        next_change: AggregatorChange,
    ) -> PartialVMResult<()> {
        // Now self follows the other delta.
        let mut prev_change = next_change;
        std::mem::swap(self, &mut prev_change);

        // Perform the merge.
        self.merge_with_previous_aggregator_change(prev_change)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bounded_math::DeltaValue;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_merge_aggregator_data_into_delta() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 20 },
            base_aggregator: None,
        };

        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(10),
                delta: DeltaValue::Positive(10),
                history: DeltaHistory {
                    max_achieved_positive_delta: 50,
                    min_achieved_negative_delta: 5,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        };
        let mut aggregator_change3 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(40),
                delta: DeltaValue::Positive(10),
                history: DeltaHistory {
                    max_achieved_positive_delta: 50,
                    min_achieved_negative_delta: 35,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        };

        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 30 },
            base_aggregator: None,
        });
        assert_err!(aggregator_change3.merge_with_previous_aggregator_change(aggregator_change2));
    }

    #[test]
    fn test_merge_data_into_data() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 20 },
            base_aggregator: None,
        };

        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 50 },
            base_aggregator: None,
        };

        let mut aggregator_change3 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 70 },
            base_aggregator: None,
        };

        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 50 },
            base_aggregator: None,
        });
        assert_ok!(aggregator_change3.merge_with_previous_aggregator_change(aggregator_change2));
        assert_eq!(aggregator_change3, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Data { value: 70 },
            base_aggregator: None,
        });
    }

    #[test]
    fn test_merge_delta_into_delta() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(20),
                delta: DeltaValue::Positive(10),
                history: DeltaHistory {
                    max_achieved_positive_delta: 30,
                    min_achieved_negative_delta: 15,
                    min_overflow_positive_delta: Some(90),
                    max_underflow_negative_delta: Some(25),
                },
            },
            base_aggregator: None,
        };
        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(40),
                delta: DeltaValue::Positive(20),
                history: DeltaHistory {
                    max_achieved_positive_delta: 25,
                    min_achieved_negative_delta: 20,
                    min_overflow_positive_delta: Some(95),
                    max_underflow_negative_delta: Some(45),
                },
            },
            base_aggregator: None,
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::Unset,
                delta: DeltaValue::Positive(30),
                history: DeltaHistory {
                    max_achieved_positive_delta: 35,
                    min_achieved_negative_delta: 15,
                    min_overflow_positive_delta: Some(90),
                    max_underflow_negative_delta: Some(25),
                },
            },
            base_aggregator: None,
        });
    }

    #[test]
    fn test_merge_delta_into_delta2() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(70),
                delta: DeltaValue::Negative(40),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: Some(40),
                    max_underflow_negative_delta: Some(80),
                },
            },
            base_aggregator: None,
        };
        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(60),
                delta: DeltaValue::Negative(20),
                history: DeltaHistory {
                    max_achieved_positive_delta: 35,
                    min_achieved_negative_delta: 20,
                    min_overflow_positive_delta: Some(85),
                    max_underflow_negative_delta: Some(75),
                },
            },
            base_aggregator: None,
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::Unset,
                delta: DeltaValue::Negative(60),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: Some(40),
                    max_underflow_negative_delta: Some(80),
                },
            },
            base_aggregator: None,
        });
        let mut aggregator_change3 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(80),
                delta: DeltaValue::Positive(5),
                history: DeltaHistory {
                    max_achieved_positive_delta: 5,
                    min_achieved_negative_delta: 5,
                    min_overflow_positive_delta: Some(91),
                    max_underflow_negative_delta: Some(95),
                },
            },
            base_aggregator: None,
        };
        assert_ok!(aggregator_change3.merge_with_previous_aggregator_change(aggregator_change2));
        assert_eq!(aggregator_change3, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::Unset,
                delta: DeltaValue::Negative(55),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 65,
                    min_overflow_positive_delta: Some(31),
                    max_underflow_negative_delta: Some(80),
                },
            },
            base_aggregator: None,
        });
    }

    #[test]
    fn test_merge_delta_into_delta3() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(70),
                delta: DeltaValue::Positive(20),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        };
        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(10),
                delta: DeltaValue::Negative(5),
                history: DeltaHistory {
                    max_achieved_positive_delta: 10,
                    min_achieved_negative_delta: 5,
                    min_overflow_positive_delta: Some(95),
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::Unset,
                delta: DeltaValue::Positive(15),
                history: DeltaHistory {
                    max_achieved_positive_delta: 30,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        });
    }

    #[test]
    fn test_merge_delta_into_delta4() {
        let aggregator_change1 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::AggregatedValue(70),
                delta: DeltaValue::Negative(20),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        };
        let mut aggregator_change2 = AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::LastCommittedValue(70),
                delta: DeltaValue::Positive(5),
                history: DeltaHistory {
                    max_achieved_positive_delta: 10,
                    min_achieved_negative_delta: 5,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: Some(90),
                },
            },
            base_aggregator: None,
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange {
            max_value: 100,
            state: AggregatorState::Delta {
                speculative_start_value: SpeculativeStartValue::Unset,
                delta: DeltaValue::Negative(15),
                history: DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            },
            base_aggregator: None,
        });
    }
}
