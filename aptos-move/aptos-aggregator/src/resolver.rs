// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::SignedU128,
    delta_change_set::{serialize, DeltaOp},
    types::{DelayedFieldID, DelayedFieldValue, DelayedFieldsSpeculativeError, PanicOr},
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadataKind},
    },
    write_set::WriteOp,
};
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::vm_status::{StatusCode, VMStatus};

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
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_aggregator_v1_value(&self, id: &Self::IdentifierV1) -> anyhow::Result<Option<u128>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
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
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    /// Fetch a value of a DelayedField.
    fn get_delayed_field_value(
        &self,
        id: &Self::IdentifierV2,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>>;

    /// Fetch an outcome of whether additional delta can be applied.
    /// `base_delta` argument represents a cumulative value that we previously checked,
    /// and `delta` argument represents a new increment.
    /// (This allows method to be stateless, and not require it to store previous calls,
    /// i.e. for sequential execution)
    ///
    /// For example, calls would go like this:
    /// try_add_delta_outcome(base_delta = 0, delta = 5) -> true
    /// try_add_delta_outcome(base_delta = 5, delta = 3) -> true
    /// try_add_delta_outcome(base_delta = 8, delta = 2) -> false
    /// try_add_delta_outcome(base_delta = 8, delta = 3) -> false
    /// try_add_delta_outcome(base_delta = 8, delta = -3) -> true
    /// try_add_delta_outcome(base_delta = 5, delta = 2) -> true
    /// ...
    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::IdentifierV2,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>>;

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
    ) -> anyhow::Result<WriteOp, VMStatus> {
        let base = self
            .get_aggregator_v1_value(id)
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

impl<S> TDelayedFieldView for S
where
    S: StateView,
{
    type IdentifierV1 = StateKey;
    type IdentifierV2 = DelayedFieldID;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }

    fn get_delayed_field_value(
        &self,
        _id: &Self::IdentifierV2,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        // TODO check if any of these methods need to be implemented
        unimplemented!("get_delayed_field_value not implemented")
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        _id: &Self::IdentifierV2,
        _base_delta: &SignedU128,
        _delta: &SignedU128,
        _max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        unimplemented!("delayed_field_try_add_delta_outcome not implemented")
    }

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator V2.
    fn generate_delayed_field_id(&self) -> Self::IdentifierV2 {
        unimplemented!("generate_delayed_field_id not implemented")
    }
}
