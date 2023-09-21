// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta_change_set::DeltaOp,
    types::{
        code_invariant_error, AggregatorValue, DelayedFieldsSpeculativeError, PanicOr,
        PanicOrResult, SnapshotToStringFormula,
    },
};

// TODO To be renamed to DelayedApplyChange
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

// TODO To be renamed to DelayedChange
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregatorChange<I: Clone> {
    Create(AggregatorValue),
    Apply(AggregatorApplyChange<I>),
}

// Contains information on top of which value should AggregatorApplyChange be applied.
pub enum ApplyBase<I: Clone> {
    // Apply on top of the value end the end of the previous transaction
    // (basically value at the start of the transaction.
    // all changes in this transaction are captured in the Apply itself)
    Previous(I),
    // Apply on top of the value at the end of the current transaction
    // I.e. if this transaction changes the aggregator under wrapped ID,
    // that apply needs to be applied first, before the current one is applied.
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

    pub fn apply_to_base(
        &self,
        base_value: AggregatorValue,
    ) -> PanicOrResult<AggregatorValue, DelayedFieldsSpeculativeError> {
        use AggregatorApplyChange::*;

        Ok(match self {
            AggregatorDelta { delta } => {
                AggregatorValue::Aggregator(delta.apply_to(base_value.into_aggregator_value()?)?)
            },
            SnapshotDelta { delta, .. } => {
                AggregatorValue::Snapshot(delta.apply_to(base_value.into_aggregator_value()?)?)
            },
            SnapshotDerived { formula, .. } => {
                AggregatorValue::Derived(formula.apply_to(base_value.into_snapshot_value()?))
            },
        })
    }
}

impl<I: Copy + Clone> AggregatorChange<I> {
    // When squashing a new change on top of the old one, sometimes we need to know the change
    // from a different AggregatorID to be able to merge them together.
    // In particular SnapshotDelta represents a change from the aggregator at the beginning of the transaction,
    // and squashing changes where the aggregator will be at the beginning of the transaction.
    // For example, let’s say we have two change sets that we need to squash:
    // change1: agg1 -> Delta(+3)
    // change2: agg1 -> Delta(+6), snap1 -> (base=agg1, Delta(+2))
    // the correct squashing of snapshot depends on the change for the base aggregator. I.e. the correct output would be:
    // agg1 -> Delta(+9), snap(base=agg1, Delta(+5))
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

    /// Applies next AggregatorChange on top of the previous state.
    /// If get_merge_dependent_id() returns some add, prev_change passed in should be
    /// for that particular AggregatorID.
    /// If get_merge_dependent_id() returns None, prev_change is required to be for the
    /// same AggregatorID as next_change.
    pub fn merge_two_changes(
        prev_change: Option<&AggregatorChange<I>>,
        next_change: &AggregatorChange<I>,
    ) -> PanicOrResult<AggregatorChange<I>, DelayedFieldsSpeculativeError> {
        use AggregatorApplyChange::*;
        use AggregatorChange::*;
        use AggregatorValue::*;

        // There are only few valid cases for merging:
        // - next_change being AggregatorDelta, and prev_change being Aggregator Create or Delta
        // - next_change being SnapshotDelta, and prev_dependent_change being Aggregator Create or Delta
        // everything else is invalid for various reasons
        match (&prev_change, next_change) {
            (None, v) => Ok(v.clone()),
            (_, Create(_)) => Err(PanicOr::from(code_invariant_error(
                "Trying to merge Create with an older change. Create should always be the first change.",
            ))),

            // Aggregators:
            (Some(Create(Aggregator(prev_value))), Apply(AggregatorDelta { delta: next_delta })) => {
                let new_data = next_delta.apply_to(*prev_value)?;
                Ok(Create(Aggregator(new_data)))
            },
            (Some(Apply(AggregatorDelta { delta: prev_delta })), Apply(AggregatorDelta { delta: next_delta })) => {
                let new_delta = DeltaOp::create_merged_delta(prev_delta, next_delta)?;
                Ok(Apply(AggregatorDelta { delta: new_delta }))
            },

            // Snapshots:
            (Some(Create(Snapshot(_) | Derived(_)) | Apply(SnapshotDelta {..} | SnapshotDerived { .. })), _) => Err(PanicOr::from(code_invariant_error(
                "Snapshots are immutable, previous change cannot be any of the snapshots type",
            ))),
            (_, Apply(SnapshotDerived { .. })) => Err(PanicOr::from(code_invariant_error(
                "Trying to merge SnapshotDerived with an older change. Snapshots are immutable, should only ever have one change.",
            ))),
            (Some(Create(Aggregator(prev_value))), Apply(SnapshotDelta { delta: next_delta, .. })) => {
                let new_data = next_delta.apply_to(*prev_value)?;
                Ok(Create(Snapshot(new_data)))
            },
            (Some(Apply(AggregatorDelta { delta: prev_delta })), Apply(SnapshotDelta { delta: next_delta, base_aggregator })) => {
                let new_delta = DeltaOp::create_merged_delta(prev_delta, next_delta)?;
                Ok(Apply(SnapshotDelta { delta: new_delta, base_aggregator: *base_aggregator }))
            },
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
        let aggregator_change1: AggregatorChange<AggregatorID> = Create(Aggregator(20));

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

        let result =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &aggregator_change2);
        assert_ok!(&result);
        let merged = result.unwrap();

        assert_eq!(merged, Create(Aggregator(30)));
        assert_err!(AggregatorChange::merge_two_changes(
            Some(&merged),
            &aggregator_change3
        ));
    }

    #[test]
    fn test_merge_data_into_data() {
        let aggregator_change1: AggregatorChange<AggregatorID> = Create(Aggregator(20));
        let aggregator_change2 = Create(Aggregator(50));
        let aggregator_change3 = Create(Aggregator(70));

        assert_err!(AggregatorChange::merge_two_changes(
            Some(&aggregator_change1),
            &aggregator_change2
        ));
        assert_err!(AggregatorChange::merge_two_changes(
            Some(&aggregator_change2),
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

        let result =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &aggregator_change2);
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

        let result_1 =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &aggregator_change2);
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

        let result_2 = AggregatorChange::merge_two_changes(Some(&merged_1), &aggregator_change3);
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
        let result =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &aggregator_change2);
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
        let result =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &aggregator_change2);
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

    #[test]
    fn test_merge_two_changes_with_dependent_change() {
        let aggregator_change1 = Apply(AggregatorDelta {
            delta: DeltaOp::new(SignedU128::Positive(3), 100, DeltaHistory {
                max_achieved_positive_delta: 3,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: Some(10),
                max_underflow_negative_delta: None,
            }),
        });
        let snapshot_change_2 = Apply(SnapshotDelta {
            base_aggregator: AggregatorID::new(1),
            delta: DeltaOp::new(SignedU128::Positive(2), 100, DeltaHistory {
                max_achieved_positive_delta: 6,
                min_achieved_negative_delta: 0,
                min_overflow_positive_delta: Some(8),
                max_underflow_negative_delta: None,
            }),
        });

        let result =
            AggregatorChange::merge_two_changes(Some(&aggregator_change1), &snapshot_change_2);
        assert_ok!(&result);

        assert_eq!(
            result.unwrap(),
            Apply(SnapshotDelta {
                base_aggregator: AggregatorID::new(1),
                delta: DeltaOp::new(SignedU128::Positive(5), 100, DeltaHistory {
                    max_achieved_positive_delta: 9,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: Some(10),
                    max_underflow_negative_delta: None,
                },)
            })
        );
    }
}
