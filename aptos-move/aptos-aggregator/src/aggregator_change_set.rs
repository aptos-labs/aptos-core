// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::code_invariant_error,
    delta_change_set::DeltaOp,
    types::{AggregatorValue, SnapshotToStringFormula},
};
use move_binary_format::errors::PartialVMResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorApplyChange<I: Clone> {
    AggregatorDelta {
        delta: DeltaOp,
    },
    /// Value is:
    /// (value of base_aggregator at the BEGINNING of the transaction + delta)
    SnapshotDelta {
        base_aggregator: I,
        delta: DeltaOp,
    },
    /// Value is:
    /// formula(value of base_snapshot at the END of the transaction)
    SnapshotDerived {
        base_snapshot: I,
        formula: SnapshotToStringFormula,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorChange<I: Clone> {
    Create(AggregatorValue),
    Apply(AggregatorApplyChange<I>),
}

pub enum ApplyBase<I: Clone> {
    Previous(I),
    Current(I),
}

impl<I: Copy + Clone> AggregatorApplyChange<I> {
    pub fn get_apply_base_id_option(&self) -> Option<ApplyBase<I>> {
        use AggregatorApplyChange::*;

        match self {
            AggregatorDelta { .. } => None,
            SnapshotDelta {
                base_aggregator, ..
            } => Some(ApplyBase::Previous(*base_aggregator)),
            SnapshotDerived { base_snapshot, .. } => Some(ApplyBase::Current(*base_snapshot)),
        }
    }

    pub fn get_apply_base_id(&self, self_id: &I) -> ApplyBase<I> {
        self.get_apply_base_id_option()
            .unwrap_or(ApplyBase::Previous(*self_id))
    }

    pub fn apply_to_base(&self, base_value: AggregatorValue) -> PartialVMResult<AggregatorValue> {
        use AggregatorApplyChange::*;

        // currently all "applications" are on top of int value, so we can do this once
        let base_value_int = base_value.into_integer_value()?;
        Ok(match self {
            AggregatorDelta { delta } => AggregatorValue::Integer(delta.apply_to(base_value_int)?),
            SnapshotDelta { delta, .. } => {
                AggregatorValue::Integer(delta.apply_to(base_value_int)?)
            },
            SnapshotDerived { formula, .. } => {
                AggregatorValue::String(formula.apply(base_value_int))
            },
        })
    }
}

impl<I: Copy + Clone> AggregatorChange<I> {
    pub fn get_merge_dependent_id(&self) -> Option<I> {
        use AggregatorApplyChange::*;
        use AggregatorChange::*;

        match self {
            // Only SnapshotDelta merging logic depends on current aggregator change
            Apply(SnapshotDelta {
                base_aggregator, ..
            }) => Some(*base_aggregator),
            Create(_) | Apply(AggregatorDelta { .. } | SnapshotDerived { .. }) => None,
        }
    }

    /// Applies next aggregator change on top of self, merging two changes together.
    pub fn merge_two_changes(
        prev_change: Option<&AggregatorChange<I>>,
        prev_dependent_change: Option<&AggregatorChange<I>>,
        next_change: &AggregatorChange<I>,
    ) -> PartialVMResult<AggregatorChange<I>> {
        use AggregatorApplyChange::*;
        use AggregatorChange::*;
        use AggregatorValue::*;

        // There are only few valid cases for merging:
        // - next_change being AggregatorDelta, and prev_change being Aggregator Create or Delta
        // - next_change being SnapshotDelta, and prev_dependent_change being Aggregator Create or Delta
        // everything else is invalid for various reasons
        match (&prev_change, &prev_dependent_change, next_change) {
            (None, None, _) => unreachable!("We should be only merging, if there is something to merge"),
            (_ , _, Create(_)) => Err(code_invariant_error(
                "Trying to merge Create with an older change. Create should always be the first change.",
            )),

            // Aggregators:
            (Some(Create(Integer(prev_value))), None, Apply(AggregatorDelta { delta: next_delta })) => {
                let new_data = next_delta.apply_to(*prev_value)?;
                Ok(Create(Integer(new_data)))
            },
            (Some(Apply(AggregatorDelta { delta: prev_delta })), None, Apply(AggregatorDelta { delta: next_delta })) => {
                let new_delta = DeltaOp::create_merged_delta(prev_delta, next_delta)?;
                Ok(Apply(AggregatorDelta { delta: new_delta }))
            },

            // Snapshots:
            (Some(Create(Integer(_) | String(_)) | Apply(SnapshotDelta {..} | SnapshotDerived { .. })), _, _) => Err(code_invariant_error(
                "Snapshots are immutable, previous change cannot be any of the snapshots type",
            )),
            (_, Some(_), Apply(AggregatorDelta { .. } | SnapshotDerived { .. })) =>
                unreachable!("Only SnapshotDelta should have merge dependent changes"),
            (_, _, Apply(SnapshotDerived { .. })) => Err(code_invariant_error(
                "Trying to merge SnapshotDerived with an older change. Snapshots are immutable, should only ever have one change.",
            )),
            (Some(_), _, Apply(SnapshotDelta { .. })) => Err(code_invariant_error(
                "Trying to merge Snapshot (delta or derived) with an older change on the same ID. Snapshots are immutable, should only ever have one change - that creates them",
            )),
            (None, Some(Create(Integer(prev_value))), Apply(SnapshotDelta { delta: next_delta, .. })) => {
                let new_data = next_delta.apply_to(*prev_value)?;
                Ok(Create(Integer(new_data)))
            }
            (None, Some(Apply(AggregatorDelta { delta: prev_delta })), Apply(SnapshotDelta { delta: next_delta, base_aggregator })) => {
                let new_delta = DeltaOp::create_merged_delta(prev_delta, next_delta)?;
                Ok(Apply(SnapshotDelta { delta: new_delta, base_aggregator: *base_aggregator }))
            }
            (None, Some(Create(String(_)) | Apply(SnapshotDelta {..} | SnapshotDerived { .. })), Apply(SnapshotDelta { .. })) => Err(code_invariant_error(
                "Trying to merge SnapshotDelta with dependent change of wrong type",
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{bounded_math::SignedU128, delta_math::DeltaHistory, types::AggregatorID};
    use claims::{assert_err, assert_ok};
    use AggregatorApplyChange::*;
    use AggregatorChange::*;
    use AggregatorValue::*;

    #[test]
    fn test_merge_aggregator_data_into_delta() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Create(Integer(20));

        let aggregator_change2 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(10), 100, DeltaHistory {
                max_achieved_positive_delta: 50,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }),
        });
        let aggregator_change3 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(10), 100, DeltaHistory {
                max_achieved_positive_delta: 50,
                min_achieved_negative_delta: 35,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }),
        });

        let result = AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2,
        );
        assert_ok!(&result);
        let merged = result.unwrap();

        assert_eq!(merged, Create(Integer(30)));
        assert_err!(AggregatorChange::merge_two_changes(
            Some(&merged),
            None,
            &aggregator_change3
        ));
    }

    #[test]
    fn test_merge_data_into_data() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Create(Integer(20));
        let aggregator_change2 = Create(Integer(50));
        let aggregator_change3 = Create(Integer(70));

        assert_err!(AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2
        ));
        assert_err!(AggregatorChange::merge_two_changes(
            Some(&aggregator_change2),
            None,
            &aggregator_change3
        ));
    }

    #[test]
    fn test_merge_delta_into_delta() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(10), 100, DeltaHistory {
                max_achieved_positive_delta: 30,
                min_achieved_negative_delta: 15,
                min_overflow_positive_delta: Some(90),
                max_underflow_negative_delta: Some(25),
            }),
        });
        let aggregator_change2 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(20), 100, DeltaHistory {
                max_achieved_positive_delta: 25,
                min_achieved_negative_delta: 20,
                min_overflow_positive_delta: Some(95),
                max_underflow_negative_delta: Some(45),
            }),
        });

        let result = AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2,
        );
        assert_ok!(&result);

        assert_eq!(
            result.unwrap(),
            Apply(AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Positive(30), 100, DeltaHistory {
                    max_achieved_positive_delta: 35,
                    min_achieved_negative_delta: 15,
                    min_overflow_positive_delta: Some(90),
                    max_underflow_negative_delta: Some(25),
                },)
            })
        );
    }

    #[test]
    fn test_merge_delta_into_delta2() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Negative(40), 100, DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 59,
                min_overflow_positive_delta: Some(40),
                max_underflow_negative_delta: Some(80),
            }),
        });
        let aggregator_change2 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Negative(20), 100, DeltaHistory {
                max_achieved_positive_delta: 35,
                min_achieved_negative_delta: 20,
                min_overflow_positive_delta: Some(85),
                max_underflow_negative_delta: Some(75),
            }),
        });

        let result_1 = AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2,
        );
        assert_ok!(&result_1);
        let merged_1 = result_1.unwrap();

        assert_eq!(
            merged_1,
            Apply(AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Negative(60), 100, DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: Some(40),
                    max_underflow_negative_delta: Some(80),
                },)
            })
        );
        let aggregator_change3 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(5), 100, DeltaHistory {
                max_achieved_positive_delta: 5,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: Some(91),
                max_underflow_negative_delta: Some(95),
            }),
        });

        let result_2 =
            AggregatorChange::merge_two_changes(Some(&merged_1), None, &aggregator_change3);
        assert_ok!(&result_2);

        assert_eq!(
            result_2.unwrap(),
            Apply(AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Negative(55), 100, DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 65,
                    min_overflow_positive_delta: Some(31),
                    max_underflow_negative_delta: Some(80),
                },)
            })
        );
    }

    #[test]
    fn test_merge_delta_into_delta3() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(20), 100, DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }),
        });
        let aggregator_change2 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Negative(5), 100, DeltaHistory {
                max_achieved_positive_delta: 10,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: Some(95),
                max_underflow_negative_delta: None,
            }),
        });
        let result = AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2,
        );
        assert_ok!(&result);

        assert_eq!(
            result.unwrap(),
            Apply(AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Positive(15), 100, DeltaHistory {
                    max_achieved_positive_delta: 30,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },)
            })
        );
    }

    #[test]
    fn test_merge_delta_into_delta4() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Negative(20), 100, DeltaHistory {
                max_achieved_positive_delta: 20,
                min_achieved_negative_delta: 60,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: None,
            }),
        });
        let aggregator_change2 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(5), 100, DeltaHistory {
                max_achieved_positive_delta: 10,
                min_achieved_negative_delta: 5,
                min_overflow_positive_delta: None,
                max_underflow_negative_delta: Some(90),
            }),
        });
        let result = AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            None,
            &aggregator_change2,
        );
        assert_ok!(&result);

        assert_eq!(
            result.unwrap(),
            Apply(AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Negative(15), 100, DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 60,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },)
            })
        );
    }
}
