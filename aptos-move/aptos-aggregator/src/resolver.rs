// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta_change_set::{serialize, DeltaOp},
    types::{DelayedFieldID, DelayedFieldValue},
};
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadataKind},
    },
    write_set::WriteOp,
};
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Defines different ways `AggregatorResolver` can be used to read its value
/// from the state.
pub enum DelayedFieldReadMode {
    /// The returned value is guaranteed to be correct.
    Aggregated,
    /// The returned value is based on last committed value, ignoring
    /// any pending changes.
    LastCommitted,
}

/// Allows to query aggregator values from the state storage.
/// Because there are two types of aggregators in the system, V1 and V2, we use
/// different code paths for each.
pub trait TDelayedFieldView {
    // We differentiate between two possible ways to identify an aggregator in
    // storage for now (V1 or V2) so that the APIs are completely separate and
    // we can delete all V1 code when necessary.
    type IdentifierV1;
    type IdentifierV2;

    /// Aggregator V1 is implemented as a state item, and therefore the API has
    /// the same pattern as for modules or resources:
    ///   -  Ok(None)         if aggregator value is not in storage,
    ///   -  Ok(Some(...))    if aggregator value exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error or failed delta
    ///                       application).
    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::IdentifierV1,
        mode: DelayedFieldReadMode,
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_aggregator_v1_value(
        &self,
        id: &Self::IdentifierV1,
        mode: DelayedFieldReadMode,
    ) -> anyhow::Result<Option<u128>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id, mode)?;
        match maybe_state_value {
            Some(state_value) => Ok(Some(bcs::from_bytes(state_value.bytes())?)),
            None => Ok(None),
        }
    }

    /// Because aggregator V1 is a state item, it also can have metadata (for
    /// example used to calculate storage refunds).
    fn get_aggregator_v1_state_value_metadata(
        &self,
        id: &Self::IdentifierV1,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // When getting state value metadata for aggregator V1, we need to do a
        // precise read.
        let maybe_state_value =
            self.get_aggregator_v1_state_value(id, DelayedFieldReadMode::Aggregated)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    fn get_delayed_field_value(
        &self,
        _id: &Self::IdentifierV2,
        _mode: DelayedFieldReadMode,
    ) -> anyhow::Result<DelayedFieldValue>;

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator V2.
    fn generate_delayed_field_id(&self) -> Self::IdentifierV2;

    /// Consumes a single delta of aggregator V1, and tries to materialize it
    /// with a given identifier (state key). If materialization succeeds, a
    /// write op is produced.
    fn try_convert_aggregator_v1_delta_into_write_op(
        &self,
        id: &Self::IdentifierV1,
        delta_op: &DeltaOp,
        mode: DelayedFieldReadMode,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        let base = self
            .get_aggregator_v1_value(id, mode)
            .map_err(|e| {
                VMStatus::error(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    Some(e.to_string()),
                )
            })?
            .ok_or_else(|| {
                VMStatus::error(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    Some("Cannot convert delta for deleted aggregator".to_string()),
                )
            })?;
        delta_op
            .apply_to(base)
            .map_err(PartialVMError::from)
            .map_err(|partial_error| partial_error.finish(Location::Undefined).into_vm_status())
            .map(|result| WriteOp::Modification(serialize(&result).into()))
    }
}

pub trait DelayedFieldResolver:
    TDelayedFieldView<IdentifierV1 = StateKey, IdentifierV2 = DelayedFieldID>
{
}

impl<T> DelayedFieldResolver for T where
    T: TDelayedFieldView<IdentifierV1 = StateKey, IdentifierV2 = DelayedFieldID>
{
}
