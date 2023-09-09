// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Scratchpad for on chain values during the execution.
//!
//! This crate provides adapters which  can be used as

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{
        get_max_binary_format_version, AptosMoveResolver, AsExecutorResolver, StateValueKind,
        StateValueMetadataKind, StateValueMetadataResolver,
    },
};
use anyhow::Error;
use aptos_aggregator::{aggregator_extension::AggregatorID, resolver::TAggregatorResolver};
use aptos_state_view::StateViewId;
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
};
use aptos_vm_types::resolver::{ExecutorResolver, StateStorageResolver};
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

/// Adapter which implements `AptosMoveResolver` using the underlying resolver.
pub struct StorageAdapter<'r, R> {
    resolver: &'r R,
    accurate_byte_count: bool,
    max_binary_format_version: u32,
    resource_group_cache:
        RefCell<BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>>,
}

pub trait AsMoveResolver<R> {
    fn as_resolver(&self) -> StorageAdapter<R>;

    fn as_resolver_with_cached_config(
        &self,
        gas_feature_version: u64,
        features: &Features,
    ) -> StorageAdapter<R>;
}

impl<R: ExecutorResolver> AsMoveResolver<R> for R {
    fn as_resolver(&self) -> StorageAdapter<R> {
        StorageAdapter::new(self)
    }

    fn as_resolver_with_cached_config(
        &self,
        gas_feature_version: u64,
        features: &Features,
    ) -> StorageAdapter<R> {
        StorageAdapter::new_with_cached_config(self, gas_feature_version, features)
    }
}

impl<'r, R: ExecutorResolver> StorageAdapter<'r, R> {
    fn new(resolver: &'r R) -> Self {
        let mut s = Self {
            resolver,
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

    fn new_with_cached_config(
        resolver: &'r R,
        gas_feature_version: u64,
        features: &Features,
    ) -> Self {
        let mut s = Self {
            resolver,
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

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> VMResult<Option<Vec<u8>>> {
        let access_path = AccessPath::resource_group_access_path(*address, resource_group.clone());
        self.resolver
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined))
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> VMResult<Option<Vec<u8>>> {
        let access_path =
            AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
            })?;
        self.resolver
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined))
    }
}

impl<'r, R: ExecutorResolver> AptosMoveResolver for StorageAdapter<'r, R> {
    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>> {
        self.resource_group_cache.take()
    }
}

impl<'r, R: ExecutorResolver> ResourceResolver for StorageAdapter<'r, R> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Vec<u8>>, usize)> {
        Ok(self.get_any_resource(address, struct_tag, metadata)?)
    }
}

impl<'r, R: ExecutorResolver> ModuleResolver for StorageAdapter<'r, R> {
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
        let access_path = AccessPath::from(module_id);
        Ok(self
            .resolver
            .get_module_bytes(&StateKey::access_path(access_path))
            .map_err(|_| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
            })?)
    }
}

impl<'r, R: ExecutorResolver> TAggregatorResolver for StorageAdapter<'r, R> {
    type Key = AggregatorID;

    fn get_aggregator_v1_state_value(&self, id: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.resolver.get_aggregator_v1_state_value(id)
    }
}

impl<'r, R: ExecutorResolver> StateValueMetadataResolver for StorageAdapter<'r, R> {
    fn get_state_value_metadata(
        &self,
        state_key: &StateKey,
        kind: StateValueKind,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        match kind {
            StateValueKind::Code => self.resolver.get_module_state_value_metadata(state_key),
            StateValueKind::Data => self.resolver.get_resource_state_value_metadata(state_key),
        }
    }
}

impl<'r, R: ExecutorResolver> TableResolver for StorageAdapter<'r, R> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.resolver
            .get_resource_bytes(&StateKey::table_item((*handle).into(), key.to_vec()), None)
    }
}

impl<'r, R: ExecutorResolver> StateStorageResolver for StorageAdapter<'r, R> {
    fn id(&self) -> StateViewId {
        self.resolver.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.resolver.get_usage()
    }
}

impl<'r, R: ExecutorResolver> ConfigStorage for StorageAdapter<'r, R> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        // Config is a resource on-chain.
        self.resolver
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .ok()?
    }
}

impl<R: ExecutorResolver> AsExecutorResolver for StorageAdapter<'_, R> {
    fn as_executor_resolver(&self) -> &dyn ExecutorResolver {
        self.resolver
    }
}
