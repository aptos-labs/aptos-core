// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::{ok_overflow, ok_underflow, BoundedMath, SignedU128},
    types::{
        DelayedFieldsSpeculativeError, DeltaApplicationFailureReason,
        DeltaHistoryMergeOffsetFailureReason,
    },
};
use aptos_types::error::{expect_ok, PanicOr};

/// Tracks values seen by aggregator. In particular, stores information about
/// the biggest and the smallest deltas that were applied successfully during
/// execution in the VM, as well as the smallest and the largest delta that failed
/// being applied. This information can be used by the executor to check if
/// final starting value would produce the same results for try_add/try_sub calls,
/// or re-execution is needed.
///  Most importantly, it allows commutativity of adds/subs. Example:
///
///
/// This graph shows how delta of aggregator changed during a single transaction
/// execution:
///
/// ```text
///                   X
///         X         :
/// +C ===========================================>
///         :         :
/// +A ===========================================>
///         :  ||     :
///         :||||     :                         +Z
///         |||||  ||||||                    ||||
///      |||||||||||||||||||||||||          |||||
/// +0 ===========================================> time
///            :          ||||||
///            :            ||
///            :            ||
/// -B ===========================================>
///            :             :
///            :             :
/// -D ===========================================>
///            X             :
///                          :
///                          X
/// ```
///
/// Clearly, +Z succeeds if +A and -B succeed, and +C and -D fail.
/// Therefore each delta validation consists of:
///   1. check +A did not overflow
///   2. check -B did not drop below zero
///   3. check +C did overflow
///   4. check -D does drop below zero
///
/// Checking +X is irrelevant since +A >= +Z, and so Z is not stored here.
#[derive(Clone, Hash, Copy, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct DeltaHistory {
    pub max_achieved_positive_delta: u128,
    pub min_achieved_negative_delta: u128,
    // `min_overflow_positive_delta` is None in two possible cases:
    // 1. No overflow occurred in the try_add/try_sub functions throughout the
    // transaction execution.
    // 2. The only overflows that occurred in the try_add/try_sub functions in
    // this transaction execution are with delta that exceeds limit.
    pub min_overflow_positive_delta: Option<u128>,
    // `max_underflow_negative_delta` is None in two possible cases:
    // 1. No underflow occurred in the try_add/try_sub functions throughout the
    // transaction execution.
    // 2. The only underflows that occurred in the try_add/try_sub functions in
    // this transaction execution are with delta that drops below -limit.
    pub max_underflow_negative_delta: Option<u128>,
}

impl std::fmt::Debug for DeltaHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "history: (")?;
        if let Some(underflow) = self.max_underflow_negative_delta {
            write!(f, "underflow: -{}, ", underflow)?;
        };
        write!(
            f,
            "achieved: [-{}, {}]",
            self.min_achieved_negative_delta, self.max_achieved_positive_delta
        )?;
        if let Some(overflow) = self.min_overflow_positive_delta {
            write!(f, ", overflow: {}", overflow)?;
        };
        Ok(())
    }
}

impl DeltaHistory {
    pub fn new() -> Self {
        DeltaHistory {
            max_achieved_positive_delta: 0,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.max_achieved_positive_delta == 0
            && self.min_achieved_negative_delta == 0
            && self.min_overflow_positive_delta.is_none()
            && self.max_underflow_negative_delta.is_none()
    }

    /// Records observed delta in history. Should be called after an operation (addition/subtraction)
    /// is successful to record its side-effects.
    pub fn record_success(&mut self, delta: SignedU128) {
        match delta {
            SignedU128::Positive(value) => {
                self.max_achieved_positive_delta =
                    u128::max(self.max_achieved_positive_delta, value)
            },
            SignedU128::Negative(value) => {
                self.min_achieved_negative_delta =
                    u128::max(self.min_achieved_negative_delta, value)
            },
        }
    }

    fn record_failure(field: &mut Option<u128>, delta: u128) {
        *field = (*field).map_or(Some(delta), |min| Some(u128::min(min, delta)));
    }

    /// Records overflows in history. Should be called after an addition is unsuccessful
    /// to record its side-effects.
    pub fn record_overflow(&mut self, delta: u128) {
        Self::record_failure(&mut self.min_overflow_positive_delta, delta);
    }

    /// Records underflows in history. Should be called after a subtraction is unsuccessful
    /// to record its side-effects.
    pub fn record_underflow(&mut self, delta: u128) {
        Self::record_failure(&mut self.max_underflow_negative_delta, delta);
    }

    /// Validates if aggregator's history is correct when applied to the `base_value`.
    /// For example, if history observed a delta of +100, and the aggregator max_value
    /// is 150, then the base value of 60 will not pass validation (60 + 100 > 150),
    /// but the base value of 30 will (30 + 100 < 150).
    /// To validate the history of an aggregator, we want to ensure that if the
    /// `base_value` is the starting value of the aggregator before the transaction
    /// execution, all the previous calls to try_add/try_sub functions returned the
    /// correct result.
    pub fn validate_against_base_value(
        &self,
        base_value: u128,
        max_value: u128,
    ) -> Result<(), DelayedFieldsSpeculativeError> {
        let math = BoundedMath::new(max_value);
        // We need to make sure the following 4 conditions are satisfied.
        //     base_value + max_achieved_positive_delta <= self.max_value
        //     base_value >= min_achieved_negative_delta
        //     base_value + min_overflow_positive_delta > self.max_value
        //     base_value < max_underflow_negative_delta
        math.unsigned_add(base_value, self.max_achieved_positive_delta)
            .map_err(|_e| DelayedFieldsSpeculativeError::DeltaApplication {
                base_value,
                max_value,
                delta: SignedU128::Positive(self.max_achieved_positive_delta),
                reason: DeltaApplicationFailureReason::Overflow,
            })?;
        math.unsigned_subtract(base_value, self.min_achieved_negative_delta)
            .map_err(|_e| DelayedFieldsSpeculativeError::DeltaApplication {
                base_value,
                max_value,
                delta: SignedU128::Negative(self.min_achieved_negative_delta),
                reason: DeltaApplicationFailureReason::Underflow,
            })?;

        if let Some(min_overflow_positive_delta) = self.min_overflow_positive_delta {
            if base_value <= max_value - min_overflow_positive_delta {
                return Err(DelayedFieldsSpeculativeError::DeltaApplication {
                    base_value,
                    max_value,
                    delta: SignedU128::Positive(min_overflow_positive_delta),
                    reason: DeltaApplicationFailureReason::ExpectedOverflow,
                });
            }
        }

        if let Some(max_underflow_negative_delta) = self.max_underflow_negative_delta {
            if base_value >= max_underflow_negative_delta {
                return Err(DelayedFieldsSpeculativeError::DeltaApplication {
                    base_value,
                    max_value,
                    delta: SignedU128::Negative(max_underflow_negative_delta),
                    reason: DeltaApplicationFailureReason::ExpectedUnderflow,
                });
            }
        }

        Ok(())
    }

    fn offset_and_merge_min_overflow(
        min_overflow: &Option<u128>,
        prev_delta: &SignedU128,
        prev_min_overflow: &Option<u128>,
        math: &BoundedMath,
    ) -> Result<Option<u128>, DelayedFieldsSpeculativeError> {
        let adjusted_min_overflow = min_overflow.map_or(
            Ok(None),
            // Return Result<Option<u128>>. we want to have None on overflow,
            // and to fail the merging on underflow
            |min_overflow| {
                ok_overflow(math.unsigned_add_delta(min_overflow, prev_delta)).map_err(|_| {
                    DelayedFieldsSpeculativeError::DeltaHistoryMergeOffset {
                        target: min_overflow,
                        delta: *prev_delta,
                        max_value: math.get_max_value(),
                        reason:
                            DeltaHistoryMergeOffsetFailureReason::FailureNotExceedingBoundsAnyMore,
                    }
                })
            },
        )?;

        Ok(match (adjusted_min_overflow, prev_min_overflow) {
            (Some(a), Some(b)) => Some(u128::min(a, *b)),
            (a, b) => a.or(*b),
        })
    }

    fn offset_and_merge_max_achieved(
        max_achieved: u128,
        prev_delta: &SignedU128,
        prev_max_achieved: u128,
        math: &BoundedMath,
    ) -> Result<u128, DelayedFieldsSpeculativeError> {
        Ok(
            ok_underflow(math.unsigned_add_delta(max_achieved, prev_delta))
                .map_err(|_| DelayedFieldsSpeculativeError::DeltaHistoryMergeOffset {
                    target: max_achieved,
                    delta: *prev_delta,
                    max_value: math.get_max_value(),
                    reason: DeltaHistoryMergeOffsetFailureReason::AchievedExceedsBounds,
                })?
                .map_or(prev_max_achieved, |value| {
                    u128::max(prev_max_achieved, value)
                }),
        )
    }

    pub fn offset_and_merge_history(
        &self,
        prev_delta: &SignedU128,
        prev_history: &Self,
        max_value: u128,
    ) -> Result<DeltaHistory, DelayedFieldsSpeculativeError> {
        let math = BoundedMath::new(max_value);

        let new_min_overflow = Self::offset_and_merge_min_overflow(
            &self.min_overflow_positive_delta,
            prev_delta,
            &prev_history.min_overflow_positive_delta,
            &math,
        )?;
        // max_underflow is identical to min_overflow, except that we offset in the opposite direction.
        let new_max_underflow = Self::offset_and_merge_min_overflow(
            &self.max_underflow_negative_delta,
            &prev_delta.minus(),
            &prev_history.max_underflow_negative_delta,
            &math,
        )?;

        // new_max_achieved = max(prev_max_achieved, max_achieved + prev_delta)
        // When adjusting max_achieved, if underflow - than the other is bigger,
        // but if overflow - we fail the merge, as we cannot successfully achieve
        // delta larger than max_value.
        let new_max_achieved = Self::offset_and_merge_max_achieved(
            self.max_achieved_positive_delta,
            prev_delta,
            prev_history.max_achieved_positive_delta,
            &math,
        )?;

        // new_min_achieved = max(prev_min_achieved, min_achieved - prev_delta)
        // Same as above, except for offsetting in the opposite direction.
        let new_min_achieved = Self::offset_and_merge_max_achieved(
            self.min_achieved_negative_delta,
            &prev_delta.minus(),
            prev_history.min_achieved_negative_delta,
            &math,
        )?;

        if new_min_overflow.is_some_and(|v| v <= new_max_achieved) {
            return Err(
                DelayedFieldsSpeculativeError::DeltaHistoryMergeAchievedAndFailureOverlap {
                    achieved: SignedU128::Positive(new_max_achieved),
                    overflow: SignedU128::Positive(new_min_overflow.unwrap()),
                },
            );
        }
        if new_max_underflow.is_some_and(|v| v <= new_min_achieved) {
            return Err(
                DelayedFieldsSpeculativeError::DeltaHistoryMergeAchievedAndFailureOverlap {
                    achieved: SignedU128::Negative(new_min_achieved),
                    overflow: SignedU128::Negative(new_max_underflow.unwrap()),
                },
            );
        }

        Ok(Self {
            max_achieved_positive_delta: new_max_achieved,
            min_achieved_negative_delta: new_min_achieved,
            min_overflow_positive_delta: new_min_overflow,
            max_underflow_negative_delta: new_max_underflow,
        })
    }

    pub fn stricter_than(&self, other: &DeltaHistory) -> bool {
        self.max_achieved_positive_delta >= other.max_achieved_positive_delta
            && self.min_achieved_negative_delta >= other.min_achieved_negative_delta
            && other.min_overflow_positive_delta.map_or(true, |other_v| {
                self.min_overflow_positive_delta
                    .is_some_and(|self_v| self_v <= other_v)
            })
            && other.max_underflow_negative_delta.map_or(true, |other_v| {
                self.max_underflow_negative_delta
                    .is_some_and(|self_v| self_v <= other_v)
            })
    }
}

pub fn merge_data_and_delta(
    prev_value: u128,
    delta: &SignedU128,
    history: &DeltaHistory,
    max_value: u128,
) -> Result<u128, PanicOr<DelayedFieldsSpeculativeError>> {
    // First, validate if the current delta operation can be applied to the base.
    history.validate_against_base_value(prev_value, max_value)?;
    // Then, apply the delta. Since history was validated, this should never fail.
    Ok(expect_ok(
        BoundedMath::new(max_value).unsigned_add_delta(prev_value, delta),
    )?)
}

pub fn merge_two_deltas(
    prev_delta: &SignedU128,
    prev_history: &DeltaHistory,
    next_delta: &SignedU128,
    next_history: &DeltaHistory,
    max_value: u128,
) -> Result<(SignedU128, DeltaHistory), PanicOr<DelayedFieldsSpeculativeError>> {
    let new_history = next_history.offset_and_merge_history(prev_delta, prev_history, max_value)?;
    let new_delta = expect_ok(BoundedMath::new(max_value).signed_add(prev_delta, next_delta))?;
    Ok((new_delta, new_history))
}

#[cfg(test)]
mod test {
    use crate::delta_math::DeltaHistory;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_change_in_base_value_1() {
        let history = DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 200,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        };
        let max_value = 600;
        assert_ok!(history.validate_against_base_value(200, max_value));
        assert_err!(history.validate_against_base_value(199, max_value));
        assert_ok!(history.validate_against_base_value(300, max_value));
        assert_err!(history.validate_against_base_value(301, max_value));
    }

    #[test]
    fn test_change_in_base_value_2() {
        let history = DeltaHistory {
            max_achieved_positive_delta: 300,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: Some(401),
            max_underflow_negative_delta: None,
        };
        let max_value = 600;
        assert_err!(history.validate_against_base_value(199, max_value));
        assert_ok!(history.validate_against_base_value(200, max_value));
        assert_ok!(history.validate_against_base_value(300, max_value));
        assert_err!(history.validate_against_base_value(301, max_value));
    }

    #[test]
    fn test_change_in_base_value_3() {
        let history = DeltaHistory {
            max_achieved_positive_delta: 200,
            min_achieved_negative_delta: 100,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: Some(201),
        };
        let max_value = 600;
        assert_ok!(history.validate_against_base_value(100, max_value));
        assert_ok!(history.validate_against_base_value(199, max_value));
        assert_ok!(history.validate_against_base_value(200, max_value));
        assert_err!(history.validate_against_base_value(201, max_value));
        assert_err!(history.validate_against_base_value(400, max_value));
    }
}
