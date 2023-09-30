// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{get_max_binary_format_version, AptosMoveResolver, AsExecutorView},
    storage_adapter::ExecutorViewBase,
};
#[allow(unused_imports)]
use anyhow::Error;
use aptos_aggregator::resolver::{AggregatorReadMode, TAggregatorView};
use aptos_state_view::{StateView, StateViewId};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    aggregator::AggregatorID,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadataKind},
    },
};
use aptos_vm_types::{
    resolver::{ExecutorView, StateStorageView, StateValueMetadataResolver, TResourceGroupView},
    resource_group_adapter::{GroupSizeKind, ResourceGroupAdapter, UnifiedResourceView},
};
use bytes::Bytes;
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver, ResourceResolver},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use paste::paste;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
};

pub(crate) fn get_resource_group_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = aptos_framework::get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

// Allows to keep a single `StorageAdapter` for both borrowed or owned views.
// For example, views are typically borrowed during block execution, but are
// owned in tests or in indexer.
// We also do not use `std::borrow::CoW` because otherwise `E` (which is the
// executor view) has to implement `Clone`.
enum ExecutorViewKind<'e, E: 'e> {
    Borrowed(&'e E),
    Owned(E),
}

impl<E> Deref for ExecutorViewKind<'_, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        match *self {
            ExecutorViewKind::Borrowed(e) => e,
            ExecutorViewKind::Owned(ref e) => e,
        }
    }
}

/// Adapter to convert a `ExecutorView` into a `AptosMoveResolver`.
///
/// Resources in groups are handled either through dedicated interfaces of executor_view
/// (that tie to specialized handling in block executor), or via 'standard' interfaces
/// for (non-group) resources and subsequent handling in the StorageAdapter itself.
pub struct StorageAdapter<'e, E> {
    // Underlying storage backend, borrowed or owned.
    executor_view: ExecutorViewKind<'e, E>,
    max_binary_format_version: u32,
    // Determines the way to compute the size of the group (based on gas version).
    group_size_kind: GroupSizeKind,
    // When block executor is true, and group_size_kind is AsBlob, we need to have an
    // alternative way to provide resource group resolution for backwards compatibility.
    // In all other cases, maybe_naive_group_view must be None.
    maybe_naive_group_view: Option<ResourceGroupAdapter<'e>>,
    accessed_groups: RefCell<HashSet<StateKey>>,
}

macro_rules! apply_to_group_view {
    ($self:ident, $f:ident ( $($p:ident),* )) => {{
	paste!(
        if let Some(group_view_override) = &$self.maybe_naive_group_view {
            group_view_override.$f($($p, )*)
        } else {
            $self.executor_view.$f($($p, )*)
        }
	    )
    }};
}

impl<'e, E: ExecutorView> StorageAdapter<'e, E> {
    pub(crate) fn from_borrowed_with_config(
        executor_view: &'e E,
        gas_feature_version: u64,
        features: &Features,
        _block_executor: bool,
    ) -> Self {
        let max_binary_version = get_max_binary_format_version(features, gas_feature_version);
        let group_size_kind = GroupSizeKind::from_gas_feature_version(gas_feature_version);
        // TODO: when resource groups are supported in block executor, do not create Adapter
        // when _block_executor = true & group_size_kind == GroupSizeKind::AsSum
        let maybe_naive_group_view = Some(ResourceGroupAdapter::from_resource_view(
            executor_view,
            GroupSizeKind::AsBlob,
        ));
        let executor_view = ExecutorViewKind::Borrowed(executor_view);

        Self::new(
            executor_view,
            max_binary_version,
            group_size_kind,
            maybe_naive_group_view,
        )
    }

    // TODO(gelash, georgemitenkov): delete after simulation uses block executor.
    pub(crate) fn from_borrowed(executor_view: &'e E, block_executor: bool) -> Self {
        let config_view = UnifiedResourceView::ResourceView(executor_view);
        let (_, gas_feature_version) = gas_config(&config_view);
        let features = Features::fetch_config(&config_view).unwrap_or_default();

        Self::from_borrowed_with_config(
            executor_view,
            gas_feature_version,
            &features,
            block_executor,
        )
    }

    fn new(
        executor_view: ExecutorViewKind<'e, E>,
        max_binary_format_version: u32,
        group_size_kind: GroupSizeKind,
        maybe_naive_group_view: Option<ResourceGroupAdapter<'e>>,
    ) -> Self {
        Self {
            executor_view,
            max_binary_format_version,
            group_size_kind,
            maybe_naive_group_view,
            accessed_groups: RefCell::new(HashSet::new()),
        }
    }

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Bytes>, usize), VMError> {
        let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            let key = StateKey::access_path(AccessPath::resource_group_access_path(
                *address,
                resource_group.clone(),
            ));

            let first_access = self.accessed_groups.borrow_mut().insert(key.clone());
            let need_size = self.group_size_kind != GroupSizeKind::None && first_access;
            let common_error = |e| -> VMError {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("{}", e))
                    .finish(Location::Undefined)
            };

            let key_ref = &key;
            let buf =
                apply_to_group_view!(self, get_resource_from_group(key_ref, struct_tag, None))
                    .map_err(common_error)?;

            let group_size = if need_size {
                apply_to_group_view!(self, resource_group_size(key_ref)).map_err(common_error)?
            } else {
                0
            };

            let buf_size = resource_size(&buf);
            Ok((buf, buf_size + group_size as usize))
        } else {
            let access_path = AccessPath::resource_access_path(*address, struct_tag.clone())
                .map_err(|_| {
                    PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
                })?;

            let buf = self
                .executor_view
                .get_resource_bytes(&StateKey::access_path(access_path), None)
                .map_err(|_| {
                    PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
                })?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }
}

impl<'e, E: ExecutorView> AptosMoveResolver for StorageAdapter<'e, E> {
    fn release_resource_group_cache(
        &self,
    ) -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>> {
        apply_to_group_view!(self, release_naive_group_cache())
    }
}

impl<'e, E: ExecutorView> ResourceResolver for StorageAdapter<'e, E> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Bytes>, usize)> {
        Ok(self.get_any_resource(address, struct_tag, metadata)?)
    }
}

impl<'e, E: ExecutorView> ModuleResolver for StorageAdapter<'e, E> {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        let module_bytes = match self.get_module(module_id) {
            Ok(Some(bytes)) => bytes,
            _ => return vec![],
        };
        let module = match CompiledModule::deserialize_with_max_version(
            &module_bytes,
            self.max_binary_format_version,
        ) {
            Ok(module) => module,
            _ => return vec![],
        };
        module.metadata
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Bytes>, Error> {
        let access_path = AccessPath::from(module_id);
        Ok(self
            .executor_view
            .get_module_bytes(&StateKey::access_path(access_path))
            .map_err(|_| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
            })?)
    }
}

impl<'e, E: ExecutorView> TableResolver for StorageAdapter<'e, E> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Bytes>, Error> {
        self.executor_view
            .get_resource_bytes(&StateKey::table_item((*handle).into(), key.to_vec()), None)
    }
}

impl<'e, E: ExecutorView> TAggregatorView for StorageAdapter<'e, E> {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::IdentifierV1,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        self.executor_view.get_aggregator_v1_state_value(id, mode)
    }
}

impl<'e, E: ExecutorView> ConfigStorage for StorageAdapter<'e, E> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Bytes> {
        self.executor_view
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .ok()?
    }
}

/// Converts `StateView` into `AptosMoveResolver`.
pub trait AsMoveResolver<S> {
    fn as_move_resolver(&self) -> StorageAdapter<ExecutorViewBase<S>>;
}

impl<S: StateView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<ExecutorViewBase<S>> {
        let config_view: UnifiedResourceView<'_, StateKey, MoveTypeLayout> =
            UnifiedResourceView::StateView(self);
        let (_, gas_feature_version) = gas_config(&config_view);
        let features = Features::fetch_config(&config_view).unwrap_or_default();

        let max_binary_version = get_max_binary_format_version(&features, gas_feature_version);
        let group_size_kind = GroupSizeKind::from_gas_feature_version(gas_feature_version);

        let executor_view =
            ExecutorViewKind::Owned(ExecutorViewBase::new(self, group_size_kind.clone()));
        StorageAdapter::new(executor_view, max_binary_version, group_size_kind, None)
    }
}

impl<'e, E: ExecutorView> StateStorageView for StorageAdapter<'e, E> {
    fn id(&self) -> StateViewId {
        self.executor_view.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.executor_view.get_usage()
    }
}

impl<'e, E: ExecutorView> StateValueMetadataResolver for StorageAdapter<'e, E> {
    fn get_module_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        self.executor_view
            .get_module_state_value_metadata(state_key)
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        self.executor_view
            .get_resource_state_value_metadata(state_key)
    }

    fn get_resource_group_state_value_metadata(
        &self,
        _state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // TODO: forward to self.executor_view.
        unimplemented!("Resource group metadata handling not yet implemented");
    }
}

// Allows to extract the view from `StorageAdapter`.
impl<'e, E: ExecutorView> AsExecutorView for StorageAdapter<'e, E> {
    fn as_executor_view(&self) -> &dyn ExecutorView {
        self.executor_view.deref()
    }
}
