// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{get_max_binary_format_version, AptosMoveResolver},
};
#[allow(unused_imports)]
use anyhow::Error;
use aptos_aggregator::{
    aggregator_extension::AggregatorID, delta_change_set::deserialize, resolver::AggregatorResolver,
};
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_state_view::StateViewId;
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
};
use aptos_vm_types::view::{StateView, StorageView, TModuleView, TResourceView};
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver, ResourceResolver},
    vm_status::StatusCode,
};
use std::{cell::RefCell, collections::BTreeMap};

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

/// Adapter to convert a `StateView` into a `MoveResolverExt`.
pub struct StorageAdapter<'a, S> {
    state_store: &'a S,
    accurate_byte_count: bool,
    max_binary_format_version: u32,
    resource_group_cache:
        RefCell<BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>>,
}

impl<'a, S: StateView> StorageAdapter<'a, S> {
    pub fn new_with_cached_config(
        state_store: &'a S,
        gas_feature_version: u64,
        features: &Features,
    ) -> Self {
        let mut s = Self {
            state_store,
            accurate_byte_count: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(BTreeMap::new()),
        };
        if gas_feature_version >= 9 {
            s.accurate_byte_count = true;
        }
        s.max_binary_format_version = get_max_binary_format_version(features, gas_feature_version);
        s
    }

    pub fn new(state_store: &'a S) -> Self {
        let mut s = Self {
            state_store,
            accurate_byte_count: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(BTreeMap::new()),
        };
        let (_, gas_feature_version) = gas_config(&s);
        let features = Features::fetch_config(&s).unwrap_or_default();
        if gas_feature_version >= 9 {
            s.accurate_byte_count = true;
        }
        s.max_binary_format_version = get_max_binary_format_version(&features, gas_feature_version);
        s
    }

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Vec<u8>>, usize), VMError> {
        let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            let mut cache = self.resource_group_cache.borrow_mut();
            let cache = cache.entry(*address).or_insert_with(BTreeMap::new);
            if let Some(group_data) = cache.get_mut(&resource_group) {
                // This resource group is already cached for this address. So just return the
                // cached value.
                let buf = group_data.get(struct_tag).cloned();
                let buf_size = resource_size(&buf);
                return Ok((buf, buf_size));
            }
            let group_data = self.get_resource_group_data(address, &resource_group)?;
            if let Some(group_data) = group_data {
                let len = if self.accurate_byte_count {
                    group_data.len()
                } else {
                    0
                };
                let group_data: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&group_data)
                    .map_err(|_| {
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .finish(Location::Undefined)
                    })?;
                let res = group_data.get(struct_tag).cloned();
                let res_size = resource_size(&res);
                cache.insert(resource_group, group_data);
                Ok((res, res_size + len))
            } else {
                cache.insert(resource_group, BTreeMap::new());
                Ok((None, 0))
            }
        } else {
            let buf = self.get_standard_resource(address, struct_tag)?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }
}

impl<'a, S: StateView> AptosMoveResolver for StorageAdapter<'a, S> {
    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let access_path = AccessPath::resource_group_access_path(*address, resource_group.clone());
        let state_key = StateKey::access_path(access_path);
        self.state_store
            .get_resource_bytes_from_view(&state_key)
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined))
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let access_path =
            AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
            })?;
        let state_key = StateKey::access_path(access_path);
        self.state_store
            .get_resource_bytes_from_view(&state_key)
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined))
    }

    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>> {
        self.resource_group_cache.take()
    }
}

impl<'a, S: StateView> ResourceResolver for StorageAdapter<'a, S> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Vec<u8>>, usize)> {
        Ok(self.get_any_resource(address, struct_tag, metadata)?)
    }
}

impl<'a, S: StateView> ModuleResolver for StorageAdapter<'a, S> {
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

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Error> {
        // REVIEW: cache this?
        let access_path = AccessPath::from(module_id);
        let state_key = StateKey::access_path(access_path);
        Ok(self
            .state_store
            .get_module_bytes_from_view(&state_key)
            .map_err(|_| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
            })?)
    }
}

impl<'a, S: StateView> TableResolver for StorageAdapter<'a, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_resource_bytes_from_view(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateView> AggregatorResolver for StorageAdapter<'a, S> {
    fn resolve_aggregator_value(&self, id: &AggregatorID) -> Result<u128, Error> {
        let AggregatorID { handle, key } = id;
        let state_key = StateKey::table_item(*handle, key.0.to_vec());
        match self.get_resource_bytes_from_view(&state_key)? {
            Some(bytes) => Ok(deserialize(&bytes)),
            None => {
                anyhow::bail!("Could not find the value of the aggregator")
            },
        }
    }
}

impl<'a, S: StateView> ConfigStorage for StorageAdapter<'a, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        let state_key = StateKey::access_path(access_path);
        self.get_resource_bytes_from_view(&state_key).ok()?
    }
}

impl<'a, S: StateView> StateStorageUsageResolver for StorageAdapter<'a, S> {
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, Error> {
        self.state_store.get_storage_usage()
    }
}

pub trait AsMoveResolver<S> {
    fn as_move_resolver(&self) -> StorageAdapter<S>;
}

impl<S: StateView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S> {
        StorageAdapter::new(self)
    }
}

impl<'a, S: StateView> TResourceView for StorageAdapter<'a, S> {
    type Key = StateKey;

    fn get_resource_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>> {
        self.state_store
            .get_resource_state_value_from_view(state_key)
    }
}

impl<'a, S: StateView> TModuleView for StorageAdapter<'a, S> {
    type Key = StateKey;

    fn get_module_state_value_from_view(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValue>> {
        self.state_store.get_module_state_value_from_view(state_key)
    }
}

impl<'a, S: StateView> StorageView for StorageAdapter<'a, S> {
    fn view_id(&self) -> StateViewId {
        self.state_store.view_id()
    }

    fn get_storage_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.state_store.get_storage_usage()
    }
}
