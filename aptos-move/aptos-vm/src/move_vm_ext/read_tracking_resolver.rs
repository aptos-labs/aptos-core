// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resolver wrapper that records the set of `StateKey`s touched by the inner
//! resolver during VM session execution. Used by the block epilogue path so the
//! VM can fold its own reads into the hot-state promotion set.

use crate::move_vm_ext::{
    resolver::{AptosMoveResolver, AsExecutorView, AsResourceGroupView, ResourceGroupResolver},
    resource_state_key,
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError},
};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    error::{PanicError, PanicOr},
    on_chain_config::ConfigStorage,
    state_store::{
        errors::StateViewError,
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId,
    },
};
use aptos_vm_types::resolver::{
    ExecutorView, ResourceGroupSize, ResourceGroupView, StateStorageView,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, metadata::Metadata,
    value::MoveTypeLayout,
};
use move_vm_types::{delayed_values::delayed_field_id::DelayedFieldID, resolver::ResourceResolver};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};
use triomphe::Arc as TriompheArc;

/// Wraps an `AptosMoveResolver`, recording every `StateKey` reachable through
/// its read APIs. The recorded set is later consumed via [`Self::into_reads`].
///
/// Tracking is best-effort and key-level: a resource access is recorded under
/// the canonical resource state key derived from its (address, struct_tag),
/// even when the resource actually lives in a resource group; group access
/// methods record the group key directly. Recording a key that doesn't
/// correspond to a live storage slot is harmless — downstream consumers (e.g.
/// the hot-state promotion logic) treat a `MakeHot` for a non-existent slot as
/// a no-op.
pub(crate) struct ReadTrackingResolver<'a, R> {
    inner: &'a R,
    reads: RefCell<BTreeSet<StateKey>>,
}

impl<'a, R: AptosMoveResolver> ReadTrackingResolver<'a, R> {
    pub fn new(inner: &'a R) -> Self {
        Self {
            inner,
            reads: RefCell::new(BTreeSet::new()),
        }
    }

    pub fn into_reads(self) -> BTreeSet<StateKey> {
        self.reads.into_inner()
    }

    fn record(&self, key: &StateKey) {
        self.reads.borrow_mut().insert(key.clone());
    }

    fn record_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> PartialVMResult<()> {
        let key = resource_state_key(address, struct_tag)?;
        self.reads.borrow_mut().insert(key);
        Ok(())
    }
}

impl<R: AptosMoveResolver> AptosMoveResolver for ReadTrackingResolver<'_, R> {}

impl<R: AptosMoveResolver> TAggregatorV1View for ReadTrackingResolver<'_, R> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        self.record(id);
        self.inner.get_aggregator_v1_state_value(id)
    }

    fn get_aggregator_v1_state_value_metadata(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        self.record(id);
        self.inner.get_aggregator_v1_state_value_metadata(id)
    }

    fn get_aggregator_v1_state_value_size(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<u64>> {
        self.record(id);
        self.inner.get_aggregator_v1_state_value_size(id)
    }
}

impl<R: AptosMoveResolver> TDelayedFieldView for ReadTrackingResolver<'_, R> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        self.inner.get_delayed_field_value(id)
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        self.inner
            .delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
    }

    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier {
        self.inner.generate_delayed_field_id(width)
    }

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError> {
        self.inner.validate_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, TriompheArc<MoveTypeLayout>)>,
        PanicError,
    > {
        self.inner
            .get_reads_needing_exchange(delayed_write_set_ids, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        self.inner
            .get_group_reads_needing_exchange(delayed_write_set_ids, skip)
    }
}

impl<R: AptosMoveResolver> ResourceResolver for ReadTrackingResolver<'_, R> {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        self.record_resource(address, struct_tag)?;
        self.inner.get_resource_bytes_with_metadata_and_layout(
            address,
            struct_tag,
            metadata,
            maybe_layout,
        )
    }
}

impl<R: AptosMoveResolver> ResourceGroupResolver for ReadTrackingResolver<'_, R> {
    fn release_resource_group_cache(
        &self,
    ) -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>> {
        self.inner.release_resource_group_cache()
    }

    fn resource_group_size(&self, group_key: &StateKey) -> PartialVMResult<ResourceGroupSize> {
        self.record(group_key);
        self.inner.resource_group_size(group_key)
    }

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<usize> {
        self.record(group_key);
        self.inner.resource_size_in_group(group_key, resource_tag)
    }

    fn resource_exists_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<bool> {
        self.record(group_key);
        self.inner.resource_exists_in_group(group_key, resource_tag)
    }
}

impl<R: AptosMoveResolver> TableResolver for ReadTrackingResolver<'_, R> {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, PartialVMError> {
        let state_key = StateKey::table_item(&(*handle).into(), key);
        self.record(&state_key);
        self.inner
            .resolve_table_entry_bytes_with_layout(handle, key, maybe_layout)
    }
}

impl<R: AptosMoveResolver> ConfigStorage for ReadTrackingResolver<'_, R> {
    fn fetch_config_bytes(&self, state_key: &StateKey) -> Option<Bytes> {
        self.record(state_key);
        self.inner.fetch_config_bytes(state_key)
    }
}

impl<R: AptosMoveResolver> StateStorageView for ReadTrackingResolver<'_, R> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.inner.id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.record(state_key);
        self.inner.read_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.inner.get_usage()
    }
}

impl<R: AptosMoveResolver> AsExecutorView for ReadTrackingResolver<'_, R> {
    fn as_executor_view(&self) -> &dyn ExecutorView {
        self.inner.as_executor_view()
    }
}

impl<R: AptosMoveResolver> AsResourceGroupView for ReadTrackingResolver<'_, R> {
    fn as_resource_group_view(&self) -> &dyn ResourceGroupView {
        self.inner.as_resource_group_view()
    }
}
