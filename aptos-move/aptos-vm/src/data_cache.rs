// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::move_vm_ext::MoveResolverExt;
#[allow(unused_imports)]
use anyhow::Error;
use aptos_framework::{natives::state_storage::StateStorageUsageResolver, RuntimeModuleMetadataV1};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::ConfigStorage,
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
};
use aptos_vm_types::remote_cache::StateViewWithRemoteCache;
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use move_table_extension::{TableHandle, TableResolver};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_types::resolver::{
    Module, ModuleRef, ModuleRefResolver, ResourceRef, ResourceRefResolver,
};
use std::ops::{Deref, DerefMut};

pub struct MoveResolverWithVMMetadata<'a, 'm, S> {
    move_resolver: &'a S,
    move_vm: &'m MoveVM,
}

impl<'a, 'm, S: MoveResolverExt> MoveResolverWithVMMetadata<'a, 'm, S> {
    pub fn new(move_resolver: &'a S, move_vm: &'m MoveVM) -> Self {
        Self {
            move_resolver,
            move_vm,
        }
    }
}

impl<'a, 'm, S: MoveResolverExt> MoveResolverExt for MoveResolverWithVMMetadata<'a, 'm, S> {
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
        aptos_framework::get_vm_metadata(self.move_vm, module_id)
    }

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        self.move_resolver
            .get_resource_group_data(address, resource_group)
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        self.move_resolver
            .get_standard_resource(address, struct_tag)
    }
}

impl<'a, 'm, S: MoveResolverExt> ModuleRefResolver for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Error = VMError;

    fn get_module_ref(&self, module_id: &ModuleId) -> Result<Option<ModuleRef>, Self::Error> {
        self.move_resolver.get_module_ref(module_id)
    }
}

impl<'a, 'm, S: MoveResolverExt> ResourceRefResolver for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Error = VMError;

    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, Self::Error> {
        // TODO: This bypasses groups!
        // self.get_any_resource(address, struct_tag)
        self.move_resolver.get_resource_ref(address, struct_tag)
    }
}

impl<'a, 'm, S: MoveResolverExt> TableResolver for MoveResolverWithVMMetadata<'a, 'm, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.move_resolver.resolve_table_entry(handle, key)
    }
}

impl<'a, 'm, S: MoveResolverExt> ConfigStorage for MoveResolverWithVMMetadata<'a, 'm, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.move_resolver.fetch_config(access_path)
    }
}

impl<'a, 'm, S: MoveResolverExt> StateStorageUsageResolver
    for MoveResolverWithVMMetadata<'a, 'm, S>
{
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, anyhow::Error> {
        self.move_resolver.get_state_storage_usage()
    }
}

impl<'a, 'm, S: MoveResolverExt> Deref for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.move_resolver
    }
}

/// Adapter to convert a `StateView` into a `MoveResolverExt`.
pub struct StorageAdapter<'a, S>(&'a S);

impl<'a, S: StateViewWithRemoteCache> StorageAdapter<'a, S> {
    pub fn new(state_store: &'a S) -> Self {
        Self(state_store)
    }

    pub fn get_resource(&self, access_path: AccessPath) -> PartialVMResult<Option<ResourceRef>> {
        self.0
            .get_move_resource(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }

    pub fn get_module(&self, access_path: AccessPath) -> PartialVMResult<Option<ModuleRef>> {
        self.0
            .get_move_module(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }
}

impl<'a, S: StateViewWithRemoteCache> MoveResolverExt for StorageAdapter<'a, S> {
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
        match self.get_module_ref(&module_id).ok()??.as_ref() {
            Module::Serialized(blob) => {
                let module = CompiledModule::deserialize(blob).ok()?;
                aptos_framework::get_metadata_from_compiled_module(&module)
            },
            Module::Cached(module) => aptos_framework::get_metadata_from_compiled_module(module),
        }
    }

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        // TODO: Currently we skip resource groups!
        panic!("Cannot call 'get_resource_group_data', - resource groups are not supported yet!")
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        let ap = AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
            PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
        })?;
        self.get_resource(ap)
            .map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateViewWithRemoteCache> ModuleRefResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_module_ref(&self, module_id: &ModuleId) -> Result<Option<ModuleRef>, Self::Error> {
        let ap = AccessPath::from(module_id);
        self.get_module(ap)
            .map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateViewWithRemoteCache> ResourceRefResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, Self::Error> {
        // TODO: Currently we skip resource groups!
        self.get_standard_resource(address, struct_tag)
    }
}

impl<'a, S: StateViewWithRemoteCache> TableResolver for StorageAdapter<'a, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_state_value_bytes(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateViewWithRemoteCache> ConfigStorage for StorageAdapter<'a, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        // TODO: Fetching of a config requires extra copy/serialization. Can be
        // expensive!
        match self.get_resource(access_path).ok()? {
            Some(r) => r.as_ref().as_bytes(),
            None => None,
        }
    }
}

impl<'a, S: StateViewWithRemoteCache> StateStorageUsageResolver for StorageAdapter<'a, S> {
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

impl<S: StateViewWithRemoteCache> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S> {
        StorageAdapter::new(self)
    }
}

/// Owned version of `StorageAdapter`.
pub struct StorageAdapterOwned<S> {
    state_view: S,
}

impl<S> Deref for StorageAdapterOwned<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state_view
    }
}

impl<S> DerefMut for StorageAdapterOwned<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state_view
    }
}

impl<S: StateViewWithRemoteCache> ModuleRefResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_module_ref(&self, module_id: &ModuleId) -> Result<Option<ModuleRef>, Self::Error> {
        self.as_move_resolver().get_module_ref(module_id)
    }
}

impl<S: StateViewWithRemoteCache> MoveResolverExt for StorageAdapterOwned<S> {
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
        self.as_move_resolver().get_module_metadata(module_id)
    }

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        self.as_move_resolver()
            .get_resource_group_data(address, resource_group)
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        self.as_move_resolver()
            .get_standard_resource(address, struct_tag)
    }
}

impl<S: StateViewWithRemoteCache> ResourceRefResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, Self::Error> {
        self.as_move_resolver()
            .get_resource_ref(address, struct_tag)
    }
}

impl<S: StateViewWithRemoteCache> TableResolver for StorageAdapterOwned<S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.as_move_resolver().resolve_table_entry(handle, key)
    }
}

impl<S: StateViewWithRemoteCache> ConfigStorage for StorageAdapterOwned<S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.as_move_resolver().fetch_config(access_path)
    }
}

impl<S: StateViewWithRemoteCache> StateStorageUsageResolver for StorageAdapterOwned<S> {
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, anyhow::Error> {
        self.as_move_resolver().get_usage()
    }
}

pub trait IntoMoveResolver<S> {
    fn into_move_resolver(self) -> StorageAdapterOwned<S>;
}

impl<S: StateViewWithRemoteCache> IntoMoveResolver<S> for S {
    fn into_move_resolver(self) -> StorageAdapterOwned<S> {
        StorageAdapterOwned { state_view: self }
    }
}
