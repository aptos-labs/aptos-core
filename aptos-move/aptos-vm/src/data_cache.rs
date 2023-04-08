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
use aptos_vm_types::{
    remote_cache::{StateViewWithRemoteCache, TRemoteCache},
    write::AptosWrite,
};
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::{ModuleResolver, ResourceResolver},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_table_extension::{TableHandle, TableResolver};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_types::{
    resolver::{Resource, ResourceResolverV2},
    values::FrozenValue,
};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

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
    ) -> Result<Option<AptosWrite>, VMError> {
        self.move_resolver
            .get_resource_group_data(address, resource_group)
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<AptosWrite>, VMError> {
        self.move_resolver
            .get_standard_resource(address, struct_tag)
    }
}

impl<'a, 'm, S: MoveResolverExt> ModuleResolver for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        self.move_resolver.get_module(module_id)
    }
}

impl<'a, 'm, S: MoveResolverExt> ResourceResolver for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.get_any_resource(address, struct_tag)
            .map(|maybe_write| {
                maybe_write.map(|write| match write {
                    AptosWrite::AggregatorValue(v) => bcs::to_bytes(&v).expect("should not fail"),
                    AptosWrite::Module(_) => unreachable!(),
                    AptosWrite::Standard(r) => r.serialize().expect("should not fail"),
                    AptosWrite::Group(_) => unreachable!(),
                })
            })
    }
}

impl<'a, 'm, S: MoveResolverExt> ResourceResolverV2 for MoveResolverWithVMMetadata<'a, 'm, S> {
    type Error = VMError;

    fn get_resource_v2(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Resource>, Self::Error> {
        self.get_any_resource(address, struct_tag)
            .map(|maybe_write| {
                maybe_write.map(|write| match write {
                    AptosWrite::AggregatorValue(v) => {
                        Resource::from_value_layout(FrozenValue::u128(v), MoveTypeLayout::U128)
                    },
                    AptosWrite::Module(_) => unreachable!(),
                    AptosWrite::Standard(r) => r,
                    AptosWrite::Group(_) => unreachable!(),
                })
            })
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

    pub fn get_r(&self, access_path: AccessPath) -> PartialVMResult<Option<AptosWrite>> {
        self.0
            .get_cached_resource(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }

    pub fn get_m(&self, access_path: AccessPath) -> PartialVMResult<Option<Vec<u8>>> {
        self.0
            .get_state_value_bytes(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }
}

impl<'a, S: StateViewWithRemoteCache> MoveResolverExt for StorageAdapter<'a, S> {
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1> {
        let module_bytes = self.get_module(&module_id).ok()??;
        let module = CompiledModule::deserialize(&module_bytes).ok()?;
        aptos_framework::get_metadata_from_compiled_module(&module)
    }

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        resource_group: &StructTag,
    ) -> Result<Option<AptosWrite>, VMError> {
        let ap = AccessPath::resource_group_access_path(*address, resource_group.clone());
        let data = self.get_r(ap).map_err(|e| e.finish(Location::Undefined));
        // TODO: fix groups
        Ok(None)
        // data.and_then(|maybe_blob| maybe_blob.and_then(|blob| {
        //     let group_data: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(blob);
        //     group_data.into_iter().map(|(t, b)| (t, Res))
        // }))
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<AptosWrite>, VMError> {
        let ap = AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
            PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
        })?;
        self.get_r(ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateViewWithRemoteCache> ModuleResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        // REVIEW: cache this?
        let ap = AccessPath::from(module_id);
        self.get_m(ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateViewWithRemoteCache> ResourceResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        panic!("Called get_resource but should not!");
        self.get_any_resource(address, struct_tag)
            .map(|maybe_write| {
                maybe_write.map(|write| match write {
                    AptosWrite::AggregatorValue(v) => unreachable!(),
                    AptosWrite::Module(_) => unreachable!(),
                    AptosWrite::Standard(r) => r.serialize().expect("should not fail"),
                    AptosWrite::Group(_) => unreachable!(),
                })
            })
    }
}

impl<'a, S: StateViewWithRemoteCache> ResourceResolverV2 for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_resource_v2(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Resource>, Self::Error> {
        self.get_any_resource(address, struct_tag)
            .map(|maybe_write| {
                maybe_write.map(|write| match write {
                    AptosWrite::AggregatorValue(v) => unreachable!(),
                    AptosWrite::Module(_) => unreachable!(),
                    AptosWrite::Standard(r) => r,
                    AptosWrite::Group(_) => unreachable!(),
                })
            })
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
        let maybe_write = self.get_r(access_path).ok()?;
        maybe_write.map(|write| match write {
            AptosWrite::Standard(r) => r.serialize().expect("should not fail"),
            _ => unreachable!(),
        })
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

impl<S: StateViewWithRemoteCache> ModuleResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_module(module_id)
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
    ) -> Result<Option<AptosWrite>, VMError> {
        self.as_move_resolver()
            .get_resource_group_data(address, resource_group)
    }

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<AptosWrite>, VMError> {
        self.as_move_resolver()
            .get_standard_resource(address, struct_tag)
    }
}

impl<S: StateViewWithRemoteCache> ResourceResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_resource(address, struct_tag)
    }
}

impl<S: StateViewWithRemoteCache> ResourceResolverV2 for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_resource_v2(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Resource>, Self::Error> {
        self.as_move_resolver().get_resource_v2(address, struct_tag)
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
