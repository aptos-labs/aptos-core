// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_extension::{AggregatorID, DerivedFormula},
    bounded_math::{code_invariant_error, expect_ok, BoundedMath, SignedU128},
    delta_math::{merge_data_and_delta, merge_two_deltas, DeltaHistory},
};
use move_binary_format::errors::PartialVMResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorChange {
    Data {
        value: u128,
    },
    StringData {
        value: Vec<u8>,
    },
    AggregatorDelta {
        delta: SignedU128,
        max_value: u128,
        history: DeltaHistory,
    },
    /// Value of delta is:
    /// formula(value of base_aggregator at the begginning of the transaction + delta)
    SnapshotDelta {
        delta: SignedU128,
        base_aggregator: AggregatorID,
        formula: DerivedFormula,
    },
}

impl AggregatorChange {
    /// Returns the result of applying the AggregatorState to the base value
    /// Returns error if the aggregator behavior based on the
    /// speculative_base_value isn't consistent with the provided base value.
    pub fn apply_aggregator_change_to(&self, base: u128) -> PartialVMResult<u128> {
        match self {
            AggregatorChange::Data { value } => Ok(*value),
            AggregatorChange::StringData { .. } => Err(code_invariant_error(
                "There should be no base value for StringData",
            )),
            AggregatorChange::AggregatorDelta {
                delta,
                history,
                max_value,
            } => merge_data_and_delta(base, delta, history, *max_value),
            AggregatorChange::SnapshotDelta { delta, .. } => {
                // History validation should already be done before this call.
                expect_ok(BoundedMath::new(u128::MAX).unsigned_add_delta(base, delta))
            },
        }
    }

    /// Applies self on top of previous change, merging them together. Note
    /// that the strict ordering here is crucial for catching overflows correctly.
    pub fn merge_with_previous_aggregator_change(
        &mut self,
        previous_change: &AggregatorChange,
    ) -> PartialVMResult<()> {
        match self {
            // If the current state is Data, then merging with previous state won't change anything.
            AggregatorChange::Data { .. } | AggregatorChange::StringData { .. } => Ok(()),
            AggregatorChange::AggregatorDelta {
                delta,
                history,
                max_value,
            } => match previous_change {
                AggregatorChange::Data { value: prev_value } => {
                    let new_data = merge_data_and_delta(*prev_value, delta, history, *max_value)?;
                    *self = AggregatorChange::Data { value: new_data };
                    Ok(())
                },
                AggregatorChange::StringData { .. } => Err(code_invariant_error(
                    "Base value cannot be string for an aggregator",
                )),
                AggregatorChange::AggregatorDelta {
                    delta: prev_delta,
                    max_value: prev_max_value,
                    history: prev_history,
                } => {
                    if max_value != prev_max_value {
                        return Err(code_invariant_error(
                            "Max value of aggregator cannot change",
                        ));
                    }
                    let (new_delta, new_history) =
                        merge_two_deltas(prev_delta, prev_history, delta, history, *max_value)?;
                    *self = AggregatorChange::AggregatorDelta {
                        delta: new_delta,
                        history: new_history,
                        max_value: *max_value,
                    };
                    Ok(())
                },
                AggregatorChange::SnapshotDelta { .. } => Err(code_invariant_error(
                    "SnapshotDelta cannot come before AggregatorDelta",
                )),
            },
            AggregatorChange::SnapshotDelta { .. } => {
                unimplemented!()
            },
        }
    }

    /// Applies next aggregator change on top of self, merging two changes together. This is a reverse
    /// of `merge_with_previous_aggregator_change`.
    pub fn merge_with_next_aggregator_change(
        &mut self,
        next_change: &AggregatorChange,
    ) -> PartialVMResult<()> {
        // Now self follows the other delta.
        let mut prev_change = next_change.clone();
        std::mem::swap(self, &mut prev_change);

        // Perform the merge.
        self.merge_with_previous_aggregator_change(&prev_change)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bounded_math::SignedU128;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_merge_aggregator_data_into_delta() {
        let aggregator_change1 = AggregatorChange::Data { value: 20 };

        let mut aggregator_change2 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(10),
            history: DeltaHistory {
                max_achieved_positive_delta: 50,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        };
        let mut aggregator_change3 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(10),
            history: DeltaHistory {
                max_achieved_positive_delta: 50,
                min_achieved_negative_delta: 35,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        };

        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::Data { value: 30 });
        assert_err!(aggregator_change3.merge_with_previous_aggregator_change(&aggregator_change2));
    }

    #[test]
    fn test_merge_data_into_data() {
        let aggregator_change1 = AggregatorChange::Data { value: 20 };

        let mut aggregator_change2 = AggregatorChange::Data { value: 50 };

        let mut aggregator_change3 = AggregatorChange::Data { value: 70 };

        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::Data { value: 50 });
        assert_ok!(aggregator_change3.merge_with_previous_aggregator_change(&aggregator_change2));
        assert_eq!(aggregator_change3, AggregatorChange::Data { value: 70 });
    }

    #[test]
    fn test_merge_delta_into_delta() {
        let aggregator_change1 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(10),
            history: DeltaHistory {
                max_achieved_positive_delta: 30,
                min_achieved_negative_delta: 15,
                min_overflow_positive_delta: Some(90),
                max_underflow_negative_delta: Some(25),
            },
        };
        let mut aggregator_change2 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(20),
            history: DeltaHistory {
                max_achieved_positive_delta: 25,
                min_achieved_negative_delta: 20,
                min_overflow_positive_delta: Some(95),
                max_underflow_negative_delta: Some(45),
            },
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(30),
            history: DeltaHistory {
                max_achieved_positive_delta: 35,
                min_achieved_negative_delta: 15,
                min_overflow_positive_delta: Some(90),
                max_underflow_negative_delta: Some(25),
            },
        });
    }

    #[test]
    fn test_merge_delta_into_delta2() {
        let aggregator_change1 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(40),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 59,
                min_overflow_positive_delta: Some(40),
                max_underflow_negative_delta: Some(80),
            },
        };
        let mut aggregator_change2 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(20),
            history: DeltaHistory {
                max_achieved_positive_delta: 35,
                min_achieved_negative_delta: 20,
                min_overflow_positive_delta: Some(85),
                max_underflow_negative_delta: Some(75),
            },
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(60),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: Some(40),
                max_underflow_negative_delta: Some(80),
            },
        });
        let mut aggregator_change3 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(5),
            history: DeltaHistory {
                max_achieved_positive_delta: 5,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: Some(91),
                max_underflow_negative_delta: Some(95),
            },
        };
        assert_ok!(aggregator_change3.merge_with_previous_aggregator_change(&aggregator_change2));
        assert_eq!(aggregator_change3, AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(55),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 65,
                min_overflow_positive_delta: Some(31),
                max_underflow_negative_delta: Some(80),
            },
        });
    }

    #[test]
    fn test_merge_delta_into_delta3() {
        let aggregator_change1 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(20),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        };
        let mut aggregator_change2 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(5),
            history: DeltaHistory {
                max_achieved_positive_delta: 10,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: Some(95),
                max_underflow_negative_delta: None,
            },
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(15),
            history: DeltaHistory {
                max_achieved_positive_delta: 30,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        });
    }

    #[test]
    fn test_merge_delta_into_delta4() {
        let aggregator_change1 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(20),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        };
        let mut aggregator_change2 = AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Positive(5),
            history: DeltaHistory {
                max_achieved_positive_delta: 10,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: Some(90),
            },
        };
        assert_ok!(aggregator_change2.merge_with_previous_aggregator_change(&aggregator_change1));
        assert_eq!(aggregator_change2, AggregatorChange::AggregatorDelta {
            max_value: 100,
            delta: SignedU128::Negative(15),
            history: DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            },
        });
    }
}
