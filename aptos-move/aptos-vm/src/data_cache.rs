// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{get_max_binary_format_version, MoveResolverExt},
};
#[allow(unused_imports)]
use anyhow::Error;
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
};
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{ModuleResolver, ResourceResolver},
    vm_status::StatusCode,
};
use move_table_extension::{TableHandle, TableResolver};
use std::{collections::BTreeMap, ops::Deref};

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

fn get_any_resource(
    move_resolver: &impl MoveResolverExt,
    address: &AccountAddress,
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Result<Option<Vec<u8>>, VMError> {
    let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
    if let Some(resource_group) = resource_group {
        let group_data = move_resolver.get_resource_group_data(address, &resource_group)?;
        if let Some(group_data) = group_data {
            let mut group_data: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&group_data)
                .map_err(|_| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .finish(Location::Undefined)
                })?;
            Ok(group_data.remove(struct_tag))
        } else {
            Ok(None)
        }
    } else {
        move_resolver.get_standard_resource(address, struct_tag)
    }
}

/// Adapter to convert a `StateView` into a `MoveResolverExt`.
pub struct StorageAdapter<'a, S>(&'a S);

impl<'a, S: StateView> StorageAdapter<'a, S> {
    pub fn new(state_store: &'a S) -> Self {
        Self(state_store)
    }

    pub fn get(&self, access_path: AccessPath) -> PartialVMResult<Option<Vec<u8>>> {
        self.0
            .get_state_value_bytes(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }
}

impl<'a, S: StateView> MoveResolverExt for StorageAdapter<'a, S> {
    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let ap = AccessPath::resource_group_access_path(*address, resource_group.clone());
        self.get(ap).map_err(|e| e.finish(Location::Undefined))
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let ap = AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
            PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
        })?;
        self.get(ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateView> ResourceResolver for StorageAdapter<'a, S> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(get_any_resource(self, address, struct_tag, metadata)?)
    }
}

impl<'a, S: StateView> ModuleResolver for StorageAdapter<'a, S> {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        let module_bytes = match self.get_module(module_id) {
            Ok(Some(bytes)) => bytes,
            _ => return vec![],
        };
        let (_, gas_feature_version) = gas_config(self);
        let features = Features::fetch_config(self).unwrap_or_default();
        let max_binary_format_version =
            get_max_binary_format_version(&features, gas_feature_version);
        let module = match CompiledModule::deserialize_with_max_version(
            &module_bytes,
            max_binary_format_version,
        ) {
            Ok(module) => module,
            _ => return vec![],
        };
        module.metadata
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Error> {
        // REVIEW: cache this?
        let ap = AccessPath::from(module_id);
        Ok(self.get(ap).map_err(|e| e.finish(Location::Undefined))?)
    }
}

impl<'a, S: StateView> TableResolver for StorageAdapter<'a, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_state_value_bytes(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateView> ConfigStorage for StorageAdapter<'a, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.get(access_path).ok()?
    }
}

impl<'a, S: StateView> StateStorageUsageResolver for StorageAdapter<'a, S> {
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, Error> {
        self.get_usage()
    }
}

impl<'a, S> Deref for StorageAdapter<'a, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.0
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
