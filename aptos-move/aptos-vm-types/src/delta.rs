// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    effects::Op,
    remote_cache::StateViewWithRemoteCache,
    write::{AptosResource, WriteOp},
};
use aptos_types::state_store::state_key::StateKey;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::{StatusCode, VMStatus},
};
use once_cell::sync::Lazy;
use std::fmt::{Debug, Formatter, Result};
use move_vm_types::resolver::Resource;
use crate::write::AptosResourceRef;

// TODO: Find a better place for these?
pub(crate) const AGGREGATOR_MODULE_IDENTIFIER: &IdentStr = ident_str!("aggregator");
pub(crate) static AGGREGATOR_MODULE: Lazy<ModuleId> =
    Lazy::new(|| ModuleId::new(CORE_CODE_ADDRESS, AGGREGATOR_MODULE_IDENTIFIER.to_owned()));

/// Error code for overflows.
const LIMIT_OVERFLOW: u64 = 0x02_0001;
/// Error code for going below zero.
const BELOW_ZERO: u64 = 0x02_0002;

/// Different delta functions.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaUpdate {
    Plus(u128),
    Minus(u128),
}

/// Represents a partial update to integers in the global state.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct DeltaOp {
    /// Maximum positive delta seen during execution.
    max_positive: u128,
    /// Smallest negative delta seen during execution.
    min_negative: u128,
    /// Post-condition: delta overflows on exceeding this limit or going below
    /// zero.
    limit: u128,
    /// Delta which is the result of the execution.
    update: DeltaUpdate,
}

/// Adds `value` to `base`. Returns error if the result of addition is greater than `limit`.
pub fn addition(base: u128, value: u128, limit: u128) -> PartialVMResult<u128> {
    if limit < base || value > (limit - base) {
        Err(abort_error(
            format!("overflow when adding {} to {}", value, base),
            LIMIT_OVERFLOW,
        ))
    } else {
        Ok(base + value)
    }
}

/// Subtracts `value` from `base`. Returns error if the result of subtraction is below zero.
pub fn subtraction(base: u128, value: u128) -> PartialVMResult<u128> {
    if value > base {
        Err(abort_error(
            format!("underflow when subtracting {} from {}", value, base),
            BELOW_ZERO,
        ))
    } else {
        Ok(base - value)
    }
}

impl DeltaOp {
    pub fn new(update: DeltaUpdate, limit: u128, max_positive: u128, min_negative: u128) -> Self {
        Self {
            max_positive,
            min_negative,
            limit,
            update,
        }
    }

    /// Returns the kind of update for this delta op.
    pub fn update(&self) -> DeltaUpdate {
        self.update
    }

    /// Returns the result of delta application to `base` or an error if
    /// the application fails.
    pub fn apply_to(&self, base: u128) -> PartialVMResult<u128> {
        // First, validate if delta op can be applied to `base`. Note that
        // this is possible if the values observed during execution didn't
        // overflow or dropped below zero. The check can be emulated by actually
        // doing addition and subtraction.
        addition(base, self.max_positive, self.limit)?;
        subtraction(base, self.min_negative)?;

        // If delta has been successfully validated, apply the update.
        match self.update {
            DeltaUpdate::Plus(value) => addition(base, value, self.limit),
            DeltaUpdate::Minus(value) => subtraction(base, value),
        }
    }

    /// Shifts the maximum positive value seen by `delta`.
    fn shifted_max_positive_by(&self, delta: &DeltaOp) -> PartialVMResult<u128> {
        match delta.update {
            // Suppose that maximum value seen is +M and we shift by +V. Then the
            // new maximum value is M+V provided addition do no overflow.
            DeltaUpdate::Plus(value) => addition(value, self.max_positive, self.limit),
            // Suppose that maximum value seen is +M and we shift by -V this time.
            // If M >= V, the result is +(M-V). Otherwise, `self` should have never
            // reached any positive value. By convention, we use 0 for the latter
            // case. Also, we can reuse `subtraction` which throws an error when M < V,
            // simply mapping the error to 0.
            DeltaUpdate::Minus(value) => Ok(subtraction(self.max_positive, value).unwrap_or(0)),
        }
    }

    /// Shifts the minimum negative value seen by `delta` .
    fn shifted_min_negative_by(&self, delta: &DeltaOp) -> PartialVMResult<u128> {
        match delta.update {
            // Suppose that minimum value seen is -M and we shift by +V. Then this case
            // is symmetric to +M-V in `shifted_max_positive_by`. Indeed, if M >= V, then
            // the minimum value should become -(M-V). Otherwise, delta had never been
            // negative and the minimum value capped to 0.
            DeltaUpdate::Plus(value) => Ok(subtraction(self.min_negative, value).unwrap_or(0)),
            // Otherwise, given  the minimum value of -M and the shift of -V the new
            // minimum value becomes -(M+V), which of course can overflow on addition,
            // implying that we subtracted too much and there was an underflow.
            DeltaUpdate::Minus(value) => addition(value, self.min_negative, self.limit),
        }
    }

    /// Applies this delta on top of the previous delta, merging them together. Note
    /// that the strict ordering here is crucial for catching errors correctly.
    pub fn merge_onto(&mut self, previous_delta: DeltaOp) -> PartialVMResult<()> {
        use DeltaUpdate::*;

        // First, update the history values of this delta given that it starts from
        // +value or -value instead of 0. We should do this check to avoid cases like this:
        //
        // Suppose we have deltas with limit of 100, and we have some `d2` which is +3 but it
        // was +99 at some point. Now, if we merge some `d1` which is +2 with `d2`, we get
        // the result is +5. However, it should not have happened because `d2` should hit
        // +2+99 > 100 at some point in history and fail.
        let shifted_max_positive = self.shifted_max_positive_by(&previous_delta)?;
        let shifted_min_negative = self.shifted_min_negative_by(&previous_delta)?;

        // Useful macro for merging deltas of the same sign, e.g. +A+B or -A-B.
        // In this cases we compute the absolute sum of deltas (A+B) and use plus
        // or minus sign accordingly.
        macro_rules! update_same_sign {
            ($sign:ident, $a:ident, $b:ident) => {
                self.update = $sign(addition($a, $b, self.limit)?)
            };
        }

        // Another useful macro, this time for merging deltas with different signs, such
        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
        // and possibly flip a sign.
        macro_rules! update_different_sign {
            ($a:ident, $b:ident) => {
                if $a >= $b {
                    self.update = Plus(subtraction($a, $b)?);
                } else {
                    self.update = Minus(subtraction($b, $a)?);
                }
            };
        }

        // History check passed, and we are ready to update the actual values now.
        match previous_delta.update {
            Plus(prev_value) => match self.update {
                Plus(self_value) => update_same_sign!(Plus, prev_value, self_value),
                Minus(self_value) => update_different_sign!(prev_value, self_value),
            },
            Minus(prev_value) => match self.update {
                Plus(self_value) => update_different_sign!(self_value, prev_value),
                Minus(self_value) => update_same_sign!(Minus, prev_value, self_value),
            },
        }

        // Deltas have been merged successfully - update the history as well.
        self.max_positive = u128::max(previous_delta.max_positive, shifted_max_positive);
        self.min_negative = u128::max(previous_delta.min_negative, shifted_min_negative);
        Ok(())
    }

    /// Consumes a single delta and tries to materialize it with a given state key.
    /// If materialization succeeds, a write is produced. Otherwise, an error is
    /// returned.
    pub fn try_materialize(
        self,
        state_view: &impl StateViewWithRemoteCache,
        state_key: &StateKey,
    ) -> anyhow::Result<Op<AptosResource>, VMStatus> {
        state_view
            .get_cached_resource(state_key)
            .map_err(|_| VMStatus::Error(StatusCode::STORAGE_ERROR, None))
            .and_then(|maybe_resource_ref| {
                match maybe_resource_ref {
                    Some(AptosResourceRef::AggregatorValue(base)) => {
                        self.apply_to(base)
                            .map_err(|partial_error| {
                                // If delta application fails, transform partial VM
                                // error into an appropriate VM status.
                                partial_error
                                    .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                                    .into_vm_status()
                            })
                            .map(|result| Op::Modification(AptosResource::AggregatorValue(result)))
                    },
                    Some(AptosResourceRef::Standard(resource)) => {
                        match resource.as_ref() {
                            Resource::Serialized(blob) => {
                                let base = bcs::from_bytes(&blob).expect("serialization of aggregator value should not fail");
                                self.apply_to(base)
                                    .map_err(|partial_error| {
                                        // If delta application fails, transform partial VM
                                        // error into an appropriate VM status.
                                        partial_error
                                            .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                                            .into_vm_status()
                                    })
                                    .map(|result| Op::Modification(AptosResource::AggregatorValue(result)))
                            },
                            Resource::Cached(_, _, _) => unreachable!("Aggregator should never be stored as a Move value")
                        }
                    },
                    // Something is wrong, the value to which we apply delta should
                    // always exist. Guard anyway.
                    None => Err(VMStatus::Error(StatusCode::STORAGE_ERROR, None)),
                }
            })
    }
}

impl Debug for DeltaOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.update {
            DeltaUpdate::Plus(value) => {
                write!(
                    f,
                    "+{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
            },
            DeltaUpdate::Minus(value) => {
                write!(
                    f,
                    "-{} ensures 0 <= result <= {}, range [-{}, {}]",
                    value, self.limit, self.min_negative, self.max_positive
                )
            },
        }
    }
}

// Helper for tests, #[cfg(test)] doesn't work for cross-crate.
pub fn delta_sub(v: u128, limit: u128) -> DeltaOp {
    DeltaOp::new(DeltaUpdate::Minus(v), limit, 0, v)
}

// Helper for tests, #[cfg(test)] doesn't work for cross-crate.
pub fn delta_add(v: u128, limit: u128) -> DeltaOp {
    DeltaOp::new(DeltaUpdate::Plus(v), limit, v, 0)
}

/// Returns partial VM error on abort. Can be used to return descriptive error messages and
/// an appropriate error code.
fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}
