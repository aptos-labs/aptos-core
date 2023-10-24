// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v1_extension::{addition_v1_error, subtraction_v1_error},
    bounded_math::SignedU128,
    delta_change_set::{serialize, DeltaOp},
    module::AGGREGATOR_MODULE,
    types::{
        code_invariant_error, DelayedFieldID, DelayedFieldValue, DelayedFieldsSpeculativeError,
        DeltaApplicationFailureReason, PanicOr,
    },
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadataKind},
    },
    write_set::WriteOp,
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{StatusCode, VMStatus};

/// We differentiate between deprecated way to interact with aggregators (TAggregatorV1View),
/// and new, more general, TDelayedFieldView.

/// Allows to query AggregatorV1 values from the state storage.
pub trait TAggregatorV1View {
    type Identifier;

    /// Aggregator V1 is implemented as a state item, and therefore the API has
    /// the same pattern as for modules or resources:
    ///   -  Ok(None)         if aggregator value is not in storage,
    ///   -  Ok(Some(...))    if aggregator value exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error or failed delta
    ///                       application).
    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_aggregator_v1_value(&self, id: &Self::Identifier) -> anyhow::Result<Option<u128>> {
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
        id: &Self::Identifier,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // When getting state value metadata for aggregator V1, we need to do a
        // precise read.
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    /// Consumes a single delta of aggregator V1, and tries to materialize it
    /// with a given identifier (state key). If materialization succeeds, a
    /// write op is produced.
    fn try_convert_aggregator_v1_delta_into_write_op(
        &self,
        id: &Self::Identifier,
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
            .map_err(|e| match &e {
                PanicOr::Or(DelayedFieldsSpeculativeError::DeltaApplication {
                    reason: DeltaApplicationFailureReason::Overflow,
                    ..
                }) => addition_v1_error(e),
                PanicOr::Or(DelayedFieldsSpeculativeError::DeltaApplication {
                    reason: DeltaApplicationFailureReason::Underflow,
                    ..
                }) => subtraction_v1_error(e),
                _ => code_invariant_error(format!("Unexpected delta application error: {:?}", e))
                    .into(),
            })
            .map_err(|partial_error| {
                partial_error
                    .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                    .into_vm_status()
            })
            .map(|result| WriteOp::Modification(serialize(&result).into()))
    }
}

pub trait AggregatorV1Resolver: TAggregatorV1View<Identifier = StateKey> {}

impl<T> AggregatorV1Resolver for T where T: TAggregatorV1View<Identifier = StateKey> {}

impl<S> TAggregatorV1View for S
where
    S: StateView,
{
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::Identifier,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }
}

/// Allows to query DelayedFields (AggregatorV2/AggregatorSnapshots) values
/// from the state storage.
pub trait TDelayedFieldView {
    type Identifier;

    fn is_delayed_field_optimization_capable(&self) -> bool;

    /// Fetch a value of a DelayedField.
    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
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
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>>;

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator V2.
    fn generate_delayed_field_id(&self) -> Self::Identifier;
}

pub trait DelayedFieldResolver: TDelayedFieldView<Identifier = DelayedFieldID> {}

impl<T> DelayedFieldResolver for T where T: TDelayedFieldView<Identifier = DelayedFieldID> {}

impl<S> TDelayedFieldView for S
where
    S: StateView,
{
    type Identifier = DelayedFieldID;

    fn is_delayed_field_optimization_capable(&self) -> bool {
        // For resolvers that are not capable, it cannot be enabled
        false
    }

    fn get_delayed_field_value(
        &self,
        _id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        unimplemented!("get_delayed_field_value not implemented")
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        _id: &Self::Identifier,
        _base_delta: &SignedU128,
        _delta: &SignedU128,
        _max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        unimplemented!("delayed_field_try_add_delta_outcome not implemented")
    }

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator V2.
    fn generate_delayed_field_id(&self) -> Self::Identifier {
        unimplemented!("generate_delayed_field_id not implemented")
    }
}
