use crate::{
    aggregator_extension::{
        validate_history, AggregatorID, AggregatorState, DeltaHistory, DeltaValue,
        SpeculativeStartValue,
    },
    delta_change_set::{abort_error, addition, subtraction},
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
                addition_deltavalue(base, delta, self.max_value)
            },
        }
    }

    /// Applies self on top of previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows
    /// correctly.
    ///
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
                        // Another useful macro, this time for merging deltas with different signs, such
                        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
                        // and possibly flip a sign.
                        macro_rules! update_different_sign {
                            ($a:ident, $b:ident) => {
                                if $a >= $b {
                                    DeltaValue::Positive(subtraction($a, $b)?)
                                } else {
                                    DeltaValue::Negative(subtraction($b, $a)?)
                                }
                            };
                        }

                        let new_delta =
                            match prev_delta {
                                DeltaValue::Positive(prev_value) => match delta {
                                    DeltaValue::Positive(self_value) => DeltaValue::Positive(
                                        addition(prev_value, self_value, self.max_value)?,
                                    ),
                                    DeltaValue::Negative(self_value) => {
                                        update_different_sign!(prev_value, self_value)
                                    },
                                },
                                DeltaValue::Negative(prev_value) => match delta {
                                    DeltaValue::Positive(self_value) => {
                                        update_different_sign!(self_value, prev_value)
                                    },
                                    DeltaValue::Negative(self_value) => DeltaValue::Negative(
                                        addition(prev_value, self_value, self.max_value)?,
                                    ),
                                },
                            };

                        // new_min_overflow = min(prev_min_overflow, prev_delta + min_overflow)
                        let new_min_overflow = match (
                            prev_history.min_overflow_positive_delta,
                            history.min_overflow_positive_delta,
                        ) {
                            (
                                Some(prev_min_overflow_positive_delta),
                                Some(min_overflow_positive_delta),
                            ) => {
                                // We know that previous_speculative_value + prev_delta + min_overflow_positive_delta is
                                // greater than max_value. Otherwise, valiate_history function should have returned an error.
                                // Therefore delta = prev_delta + min_overflow_positive_delta still results in an overflow.
                                // We are assured that prev_delta + min_overflow_positive_delta is positive. But in case
                                // prev_delta + min_overflow_positive_delta is greater than max_value, then adding
                                // delta = prev_delta + min_overflow_positive_delta to any start value will always result in
                                // an overflow. By our convention, we consider this overflow as None.
                                match addition_deltavalue(
                                    min_overflow_positive_delta,
                                    prev_delta,
                                    self.max_value,
                                ) {
                                    Ok(val) => {
                                        Some(u128::min(prev_min_overflow_positive_delta, val))
                                    },
                                    Err(_) => Some(prev_min_overflow_positive_delta),
                                }
                            },
                            (Some(prev_min_overflow_positive_delta), None) => {
                                Some(prev_min_overflow_positive_delta)
                            },
                            (None, Some(min_overflow_positive_delta)) => {
                                // We know that previous_speculative_value + prev_delta + min_overflow_positive_delta is
                                // greater than max_value. Otherwise, valiate_history function should have returned an error.
                                // Therefore delta = prev_delta + min_overflow_positive_delta still results in an overflow.
                                // We are assured that prev_delta + min_overflow_positive_delta is positive.
                                // But if prev_delta + min_overflow_positive_delta exceeds max_value, by our convention the
                                // overflow is considered as None, as any possible start value will always result in an overflow.
                                addition_deltavalue(
                                    min_overflow_positive_delta,
                                    prev_delta,
                                    self.max_value,
                                )
                                .ok()
                            },
                            (None, None) => None,
                        };

                        // new_max_underflow = min(prev_max_underflow, max_underflow - prev_delta)
                        let new_max_underflow = match (
                            prev_history.max_underflow_negative_delta,
                            history.max_underflow_negative_delta,
                        ) {
                            // We know that previous_speculative_value + prev_delta - max_underflow_negative_delta is
                            // less than 0. Otherwise, valiate_history function should have returned an error.
                            // Therefore delta = prev_delta - max_underflow_negative_delta still results in an underflow.
                            // We are assured that max_underflow_negative_delta - prev_delta is positive. But in case
                            // max_underflow_negative_delta - prev_delta exceeds max_value, then adding delta =
                            // prev_delta - max_underflow_negative_delta to any start value will always result in
                            // an underflow. By our convention, we consider this underflow as None.
                            (
                                Some(prev_max_underflow_negative_delta),
                                Some(max_underflow_negative_delta),
                            ) => match subtraction_deltavalue(
                                max_underflow_negative_delta,
                                prev_delta,
                                self.max_value,
                            ) {
                                Ok(val) => Some(u128::min(prev_max_underflow_negative_delta, val)),
                                Err(_) => Some(prev_max_underflow_negative_delta),
                            },
                            (Some(prev_max_underflow_negative_delta), None) => {
                                Some(prev_max_underflow_negative_delta)
                            },
                            (None, Some(max_underflow_negative_delta)) => {
                                // We know that previous_speculative_value + prev_delta - max_underflow_negative_delta is
                                // less than 0. Otherwise, valiate_history function should have returned an error.
                                // Therefore delta = prev_delta - max_underflow_negative_delta still results in an underflow.
                                // We are assured that max_underflow_negative_delta - prev_delta is positive. But in case
                                // max_underflow_negative_delta - prev_delta exceeds max_value, then adding delta =
                                // prev_delta - max_underflow_negative_delta to any start value will always result in
                                // an underflow. By our convention, we consider this underflow as None.
                                subtraction_deltavalue(
                                    max_underflow_negative_delta,
                                    prev_delta,
                                    self.max_value,
                                )
                                .ok()
                            },
                            (None, None) => None,
                        };

                        // new_max_achieved = max(prev_max_achieved, max_achieved + prev_delta)
                        let new_max_achieved = match prev_delta {
                            DeltaValue::Positive(prev_delta) => u128::max(
                                prev_history.max_achieved_positive_delta,
                                addition(
                                    history.max_achieved_positive_delta,
                                    prev_delta,
                                    self.max_value,
                                )?,
                            ),
                            DeltaValue::Negative(prev_delta) => {
                                if history.max_achieved_positive_delta >= prev_delta {
                                    u128::max(
                                        prev_history.max_achieved_positive_delta,
                                        subtraction(
                                            history.max_achieved_positive_delta,
                                            prev_delta,
                                        )?,
                                    )
                                } else {
                                    prev_history.max_achieved_positive_delta
                                }
                            },
                        };

                        // new_min_achieved = max(prev_min_achieved, min_achieved - prev_delta)
                        let new_min_achieved = match prev_delta {
                            DeltaValue::Positive(prev_delta) => {
                                if history.min_achieved_negative_delta >= prev_delta {
                                    u128::max(
                                        prev_history.min_achieved_negative_delta,
                                        subtraction(
                                            history.min_achieved_negative_delta,
                                            prev_delta,
                                        )?,
                                    )
                                } else {
                                    prev_history.min_achieved_negative_delta
                                }
                            },
                            DeltaValue::Negative(prev_delta) => u128::max(
                                prev_history.min_achieved_negative_delta,
                                addition(
                                    history.min_achieved_negative_delta,
                                    prev_delta,
                                    self.max_value,
                                )?,
                            ),
                        };

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

/// Implements base + value
pub fn addition_deltavalue(
    base: u128,
    value: DeltaValue,
    max_value: u128,
) -> PartialVMResult<u128> {
    match value {
        DeltaValue::Positive(value) => addition(base, value, max_value),
        DeltaValue::Negative(value) => subtraction(base, value),
    }
}

/// Implements base - value
pub fn subtraction_deltavalue(
    base: u128,
    value: DeltaValue,
    max_value: u128,
) -> PartialVMResult<u128> {
    match value {
        DeltaValue::Positive(value) => subtraction(base, value),
        DeltaValue::Negative(value) => addition(base, value, max_value),
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
