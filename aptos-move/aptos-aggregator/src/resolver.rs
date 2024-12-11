// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v1_extension::{addition_v1_error, subtraction_v1_error},
    bounded_math::SignedU128,
    delta_change_set::{serialize, DeltaOp},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError, DeltaApplicationFailureReason},
};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadata},
        StateView,
    },
    write_set::WriteOp,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

/// We differentiate between deprecated way to interact with aggregators (TAggregatorV1View),
/// and new, more general, TDelayedFieldView.

/// Allows to query AggregatorV1 values from the state storage.
pub trait TAggregatorV1View {
    type Identifier: Debug;

    /// Aggregator V1 is implemented as a state item, and therefore the API has
    /// the same pattern as for modules or resources:
    ///   -  Ok(None)         if aggregator value is not in storage,
    ///   -  Ok(Some(...))    if aggregator value exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error or failed delta
    ///                       application).
    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>>;

    fn get_aggregator_v1_value(&self, id: &Self::Identifier) -> PartialVMResult<Option<u128>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        match maybe_state_value {
            Some(state_value) => Ok(Some(bcs::from_bytes(state_value.bytes()).map_err(|e| {
                PartialVMError::new(StatusCode::UNEXPECTED_DESERIALIZATION_ERROR)
                    .with_message(format!("Failed to deserialize aggregator value: {:?}", e))
            })?)),
            None => Ok(None),
        }
    }

    /// Because aggregator V1 is a state item, it also can have metadata (for
    /// example used to calculate storage refunds).
    fn get_aggregator_v1_state_value_metadata(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        // When getting state value metadata for aggregator V1, we need to do a
        // precise read.
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    fn get_aggregator_v1_state_value_size(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<u64>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        Ok(maybe_state_value.map(|v| v.size() as u64))
    }

    /// Consumes a single delta of aggregator V1, and tries to materialize it
    /// with a given identifier (state key). If materialization succeeds, a
    /// write op is produced.
    fn try_convert_aggregator_v1_delta_into_write_op(
        &self,
        id: &Self::Identifier,
        delta_op: &DeltaOp,
    ) -> PartialVMResult<WriteOp> {
        let base = self.get_aggregator_v1_value(id)?.ok_or_else(|| {
            PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                .with_message("Cannot convert delta for deleted aggregator".to_string())
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
                // Because aggregator V1 never underflows or overflows, all other
                // application errors are bugs.
                _ => code_invariant_error(format!("Unexpected delta application error: {:?}", e))
                    .into(),
            })
            .map(|result| WriteOp::legacy_modification(serialize(&result).into()))
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
    ) -> PartialVMResult<Option<StateValue>> {
        self.get_state_value(state_key).map_err(|e| {
            PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                "Aggregator value not found for {:?}: {:?}",
                state_key, e
            ))
        })
    }
}

/// Allows to query DelayedFields (AggregatorV2/AggregatorSnapshots) values
/// from the state storage.
pub trait TDelayedFieldView {
    type Identifier;
    type ResourceKey;
    type ResourceGroupTag;

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
    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier;

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError>;

    /// Returns the list of resources that satisfy all the following conditions:
    /// 1. The resource is read during the transaction execution.
    /// 2. The resource is not present in write set of the VM Change Set.
    /// 3. The resource has a delayed field in it that is part of delayed field change set.
    /// We get the keys of these resources and metadata to include them in the write set
    /// of the transaction output after value exchange.
    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    >;

    /// Returns the list of resource groups that satisfy all the following conditions:
    /// 1. At least one of the resource in the group is read during the transaction execution.
    /// 2. The resource group is not present in the write set of the VM Change Set.
    /// 3. At least one of the resources in the group has a delayed field in it that is part.
    /// of delayed field change set.
    /// We get the keys of these resource groups and metadata to include them in the write set
    /// of the transaction output after value exchange. For each such resource group, this function
    /// outputs:(resource key, (metadata, resource group size))
    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>>;
}

pub trait DelayedFieldResolver:
    TDelayedFieldView<Identifier = DelayedFieldID, ResourceKey = StateKey, ResourceGroupTag = StructTag>
{
}

impl<T> DelayedFieldResolver for T where
    T: TDelayedFieldView<
        Identifier = DelayedFieldID,
        ResourceKey = StateKey,
        ResourceGroupTag = StructTag,
    >
{
}

impl<S> TDelayedFieldView for S
where
    S: StateView,
{
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

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
    fn generate_delayed_field_id(&self, _width: u32) -> Self::Identifier {
        unimplemented!("generate_delayed_field_id not implemented")
    }

    fn validate_delayed_field_id(&self, _id: &Self::Identifier) -> Result<(), PanicError> {
        unimplemented!()
    }

    // get_reads_needing_exchange is local (looks at in-MVHashMap information only)
    // and all failures are code invariant failures - so we return PanicError.
    // get_group_reads_needing_exchange needs to additionally get the metadata of the
    // whole group, which can additionally fail with speculative / storage errors,
    // so we return PartialVMResult, to be able to distinguish/propagate those errors.

    fn get_reads_needing_exchange(
        &self,
        _delayed_write_set_ids: &HashSet<Self::Identifier>,
        _skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        unimplemented!("get_reads_needing_exchange not implemented")
    }

    fn get_group_reads_needing_exchange(
        &self,
        _delayed_write_set_ids: &HashSet<Self::Identifier>,
        _skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        unimplemented!("get_group_reads_needing_exchange not implemented")
    }
}
