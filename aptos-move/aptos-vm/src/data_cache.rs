// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::move_vm_ext::{
    AsExecutorView, resource_state_key, AptosMoveResolver, ResourceGroupResolver,
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError},
};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    error::{PanicError, PanicOr},
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        errors::StateViewError,
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateView, StateViewId,
    },
    vm::module_metadata::get_metadata,
};
use aptos_vm_environment::gas::get_gas_feature_version;
use aptos_vm_types::{
    resolver::{
        ExecutorView, ExecutorViewWithoutGroupView, ResourceGroupSize, ResourceGroupView, StateStorageView, TResourceGroupView, TResourceView,
    },
    resource_group_adapter::ResourceGroupAdapter,
};
use bytes::Bytes;
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, metadata::Metadata,
    value::MoveTypeLayout,
};
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    resolver::{resource_size, ResourceResolver},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

pub fn get_resource_group_member_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

/// Adapter to convert a `ExecutorViewWithoutGroupView` into an `AptosMoveResolver`.
///
/// Resources in groups are handled either through dedicated interfaces of executor_view
/// (that tie to specialized handling in block executor), or via 'standard' interfaces
/// for (non-group) resources and subsequent handling in the StorageAdapter itself.
pub struct StorageAdapter<'e, E> {
    executor_view_without_group_view: &'e E,
    resource_group_view: ResourceGroupAdapter<'e>,
    accessed_groups: RefCell<HashSet<StateKey>>,
}

impl<'e, E: ExecutorViewWithoutGroupView> StorageAdapter<'e, E> {
    pub(crate) fn new_with_config(
        executor_view_without_group_view: &'e E,
        gas_feature_version: u64,
        features: &Features,
        maybe_resource_group_view: Option<&'e dyn ResourceGroupView>,
    ) -> Self {
        let resource_group_adapter = ResourceGroupAdapter::new(
            maybe_resource_group_view,
            executor_view_without_group_view,
            gas_feature_version,
            features.is_resource_groups_split_in_vm_change_set_enabled(),
        );

        Self::new(executor_view_without_group_view, resource_group_adapter)
    }

    fn new(executor_view_without_group_view: &'e E, resource_group_view: ResourceGroupAdapter<'e>) -> Self {
        Self {
            executor_view_without_group_view,
            resource_group_view,
            accessed_groups: RefCell::new(HashSet::new()),
        }
    }

    fn get_any_resource_with_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        let resource_group = get_resource_group_member_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            let key = StateKey::resource_group(address, &resource_group);
            let buf =
                self.resource_group_view
                    .get_resource_from_group(&key, struct_tag, maybe_layout)?;

            let first_access = self.accessed_groups.borrow_mut().insert(key.clone());
            let group_size = if first_access {
                self.resource_group_view.resource_group_size(&key)?.get()
            } else {
                0
            };

            let buf_size = resource_size(&buf);
            Ok((buf, buf_size + group_size as usize))
        } else {
            let state_key = resource_state_key(address, struct_tag)?;
            let buf = self
                .executor_view_without_group_view
                .get_resource_bytes(&state_key, maybe_layout)?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> ResourceGroupResolver for StorageAdapter<'e, E> {
    fn release_resource_group_cache(
        &self,
    ) -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>> {
        self.resource_group_view.release_group_cache()
    }

    fn resource_group_size(&self, group_key: &StateKey) -> PartialVMResult<ResourceGroupSize> {
        self.resource_group_view.resource_group_size(group_key)
    }

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<usize> {
        self.resource_group_view
            .resource_size_in_group(group_key, resource_tag)
    }

    fn resource_exists_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<bool> {
        self.resource_group_view
            .resource_exists_in_group(group_key, resource_tag)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> AptosMoveResolver for StorageAdapter<'e, E> {}

impl<'e, E: ExecutorViewWithoutGroupView> ResourceResolver for StorageAdapter<'e, E> {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        self.get_any_resource_with_layout(address, struct_tag, metadata, maybe_layout)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> TableResolver for StorageAdapter<'e, E> {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, PartialVMError> {
        let state_key = StateKey::table_item(&(*handle).into(), key);
        self.executor_view_without_group_view
            .get_resource_bytes(&state_key, maybe_layout)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> TAggregatorV1View for StorageAdapter<'e, E> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        self.executor_view_without_group_view.get_aggregator_v1_state_value(id)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> TDelayedFieldView for StorageAdapter<'e, E> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view_without_group_view.get_delayed_field_value(id)
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view_without_group_view
            .delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
    }

    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier {
        self.executor_view_without_group_view.generate_delayed_field_id(width)
    }

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError> {
        self.executor_view_without_group_view.validate_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        self.executor_view_without_group_view
            .get_reads_needing_exchange(delayed_write_set_keys, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        self.executor_view_without_group_view
            .get_group_reads_needing_exchange(delayed_write_set_keys, skip)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> ConfigStorage for StorageAdapter<'e, E> {
    fn fetch_config_bytes(&self, state_key: &StateKey) -> Option<Bytes> {
        self.executor_view_without_group_view
            .get_resource_bytes(state_key, None)
            .ok()?
    }
}

/// Converts `StateView` into `AptosMoveResolver`.
pub trait AsMoveResolver<S: ExecutorViewWithoutGroupView> {
    fn as_move_resolver(&self) -> StorageAdapter<S>;
}

impl<S: StateView + ExecutorViewWithoutGroupView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S> {
        let features = Features::fetch_config(self).unwrap_or_default();
        let gas_feature_version = get_gas_feature_version(self);
        let resource_group_adapter = ResourceGroupAdapter::new(
            None,
            self,
            gas_feature_version,
            features.is_resource_groups_split_in_vm_change_set_enabled(),
        );
        StorageAdapter::new(self, resource_group_adapter)
    }
}

impl<'e, E: ExecutorViewWithoutGroupView> StateStorageView for StorageAdapter<'e, E> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.executor_view_without_group_view.id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.executor_view_without_group_view.read_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.executor_view_without_group_view.get_usage()
    }
}

// Create a wrapper that can be used as an ExecutorView
struct ExecutorViewProxy<'a, 'e, E> {
    executor_view_without_group_view: &'a E,
    resource_group_view: &'a ResourceGroupAdapter<'e>,
}

impl<'a, 'e, E: ExecutorViewWithoutGroupView> ExecutorViewProxy<'a, 'e, E> {
    fn new(adapter: &'a StorageAdapter<'e, E>) -> Self {
        Self {
            executor_view_without_group_view: adapter.executor_view_without_group_view,
            resource_group_view: &adapter.resource_group_view,
        }
    }
}

// Manually implement TResourceView for ExecutorViewProxy - delegate to executor_view_without_group_view
impl<'a, 'e, E: ExecutorViewWithoutGroupView> TResourceView for ExecutorViewProxy<'a, 'e, E> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<StateValue>> {
        self.executor_view_without_group_view.get_resource_state_value(state_key, maybe_layout)
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        self.executor_view_without_group_view.get_resource_state_value_metadata(state_key)
    }
    
    fn get_resource_state_value_size(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<u64> {
        self.executor_view_without_group_view.get_resource_state_value_size(state_key)
    }

    fn resource_exists(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<bool> {
        self.executor_view_without_group_view.resource_exists(state_key)
    }
}

// Manually implement TAggregatorV1View for ExecutorViewProxy - delegate to executor_view_without_group_view
impl<'a, 'e, E: ExecutorViewWithoutGroupView> TAggregatorV1View for ExecutorViewProxy<'a, 'e, E> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        self.executor_view_without_group_view.get_aggregator_v1_state_value(id)
    }
}

// Manually implement TDelayedFieldView for ExecutorViewProxy - delegate to executor_view_without_group_view
impl<'a, 'e, E: ExecutorViewWithoutGroupView> TDelayedFieldView for ExecutorViewProxy<'a, 'e, E> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view_without_group_view.get_delayed_field_value(id)
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view_without_group_view.delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
    }

    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier {
        self.executor_view_without_group_view.generate_delayed_field_id(width)
    }

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError> {
        self.executor_view_without_group_view.validate_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        self.executor_view_without_group_view.get_reads_needing_exchange(delayed_write_set_keys, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        self.executor_view_without_group_view.get_group_reads_needing_exchange(delayed_write_set_keys, skip)
    }
}

// Manually implement StateStorageView for ExecutorViewProxy - delegate to executor_view_without_group_view
impl<'a, 'e, E: ExecutorViewWithoutGroupView> StateStorageView for ExecutorViewProxy<'a, 'e, E> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.executor_view_without_group_view.id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.executor_view_without_group_view.read_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.executor_view_without_group_view.get_usage()
    }
}

// Manually implement TResourceGroupView for ExecutorViewProxy - delegate to resource_group_view
impl<'a, 'e, E: ExecutorViewWithoutGroupView> TResourceGroupView for ExecutorViewProxy<'a, 'e, E> {
    type GroupKey = StateKey;
    type ResourceTag = StructTag;
    type Layout = MoveTypeLayout;

    fn is_resource_groups_split_in_change_set_capable(&self) -> bool {
        self.resource_group_view.is_resource_groups_split_in_change_set_capable()
    }

    fn resource_group_size(&self, group_key: &Self::GroupKey) -> PartialVMResult<ResourceGroupSize> {
        self.resource_group_view.resource_group_size(group_key)
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>> {
        self.resource_group_view.get_resource_from_group(group_key, resource_tag, maybe_layout)
    }

    fn resource_size_in_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
    ) -> PartialVMResult<usize> {
        self.resource_group_view.resource_size_in_group(group_key, resource_tag)
    }

    fn resource_exists_in_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
    ) -> PartialVMResult<bool> {
        self.resource_group_view.resource_exists_in_group(group_key, resource_tag)
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        self.resource_group_view.release_group_cache()
    }
}

// Allows to extract the view from `StorageAdapter`. This implementation is critical for the
// AptosMoveResolver trait, which requires an AsExecutorView implementation.
//
// Note that StorageAdapter itself is NOT an ExecutorView - it's a holder that properly dispatches
// queries. This intentionally forces clients to be explicit about how they're accessing the 
// underlying views and avoids unintended use.
impl<'e, E: ExecutorViewWithoutGroupView> AsExecutorView for StorageAdapter<'e, E> {
    fn as_executor_view(&self) -> &dyn ExecutorView {
        // TODO: Fix this memory allocation.
        Box::leak(Box::new(ExecutorViewProxy::new(self)))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use aptos_vm_types::resource_group_adapter::GroupSizeKind;

    // Expose a method to create a storage adapter with a provided group size kind.
    pub(crate) fn as_resolver_with_group_size_kind<S: StateView>(
        state_view: &S,
        group_size_kind: GroupSizeKind,
    ) -> StorageAdapter<S> {
        assert_ne!(group_size_kind, GroupSizeKind::AsSum, "not yet supported");

        let (gas_feature_version, resource_groups_split_in_vm_change_set_enabled) =
            match group_size_kind {
                GroupSizeKind::AsSum => (12, true),
                GroupSizeKind::AsBlob => (10, false),
                GroupSizeKind::None => (1, false),
            };

        let group_adapter = ResourceGroupAdapter::new(
            // TODO[agg_v2](test) add a converter for StateView for tests that implements ResourceGroupView
            None,
            state_view,
            gas_feature_version,
            resource_groups_split_in_vm_change_set_enabled,
        );

        StorageAdapter::new(state_view, group_adapter)
    }
}
