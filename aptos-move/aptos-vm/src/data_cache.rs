// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::create_access_path;
#[allow(unused_imports)]
use anyhow::format_err;
use anyhow::Error;
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::ConfigStorage,
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    vm_status::StatusCode,
};
use framework::natives::state_storage::StateStorageUsageResolver;
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::{ModuleResolver, ResourceResolver},
};
use move_table_extension::{TableHandle, TableResolver};
use std::ops::{Deref, DerefMut};

// Adapter to convert a `StateView` into a `RemoteCache`.
pub struct StorageAdapter<'a, S>(&'a S);

impl<'a, S: StateView<StateKey>> StorageAdapter<'a, S> {
    pub fn new(state_store: &'a S) -> Self {
        Self(state_store)
    }

    pub fn get(&self, access_path: &AccessPath) -> PartialVMResult<Option<Vec<u8>>> {
        self.0
            .get_state_value(&StateKey::AccessPath(access_path.clone()))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }
}

impl<'a, S: StateView<StateKey>> ModuleResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        // REVIEW: cache this?
        let ap = AccessPath::from(module_id);
        self.get(&ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateView<StateKey>> ResourceResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let ap = create_access_path(*address, struct_tag.clone());
        self.get(&ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateView<StateKey>> TableResolver for StorageAdapter<'a, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_state_value(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateView<StateKey>> ConfigStorage for StorageAdapter<'a, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.get(&access_path).ok()?
    }
}

impl<'a, S: StateView<StateKey>> StateStorageUsageResolver for StorageAdapter<'a, S> {
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

impl<S: StateView<StateKey>> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S> {
        StorageAdapter::new(self)
    }
}

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

impl<S: StateView<StateKey>> ModuleResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_module(module_id)
    }
}

impl<S: StateView<StateKey>> ResourceResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_resource(address, struct_tag)
    }
}

impl<S: StateView<StateKey>> TableResolver for StorageAdapterOwned<S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.as_move_resolver().resolve_table_entry(handle, key)
    }
}

impl<S: StateView<StateKey>> ConfigStorage for StorageAdapterOwned<S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.as_move_resolver().fetch_config(access_path)
    }
}

impl<S: StateView<StateKey>> StateStorageUsageResolver for StorageAdapterOwned<S> {
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, anyhow::Error> {
        self.as_move_resolver().get_usage()
    }
}

pub trait IntoMoveResolver<S> {
    fn into_move_resolver(self) -> StorageAdapterOwned<S>;
}

impl<S: StateView<StateKey>> IntoMoveResolver<S> for S {
    fn into_move_resolver(self) -> StorageAdapterOwned<S> {
        StorageAdapterOwned { state_view: self }
    }
}
