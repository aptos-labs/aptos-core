// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{counters::CRITICAL_ERRORS, create_access_path, logging::AdapterLogSchema};
#[allow(unused_imports)]
use anyhow::format_err;
use anyhow::Error;
use aptos_logger::prelude::*;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::ConfigStorage,
    state_store::state_key::StateKey,
    vm_status::StatusCode,
    write_set::{WriteOp, WriteSet},
};
use fail::fail_point;
use framework::natives::state_storage::StateStorageUsageResolver;
use move_deps::{
    move_binary_format::errors::*,
    move_core_types::{
        account_address::AccountAddress,
        language_storage::{ModuleId, StructTag},
        resolver::{ModuleResolver, ResourceResolver},
    },
    move_table_extension::{TableHandle, TableResolver},
};
use std::{
    collections::btree_map::BTreeMap,
    ops::{Deref, DerefMut},
};

/// A local cache for a given a `StateView`. The cache is private to the Aptos layer
/// but can be used as a one shot cache for systems that need a simple `RemoteCache`
/// implementation (e.g. tests or benchmarks).
///
/// The cache is responsible to track all changes to the `StateView` that are the result
/// of transaction execution. Those side effects are published at the end of a transaction
/// execution via `StateViewCache::push_write_set`.
///
/// `StateViewCache` is responsible to give an up to date view over the data store,
/// so that changes executed but not yet committed are visible to subsequent transactions.
///
/// If a system wishes to execute a block of transaction on a given view, a cache that keeps
/// track of incremental changes is vital to the consistency of the data store and the system.
pub struct StateViewCache<'a, S> {
    data_view: &'a S,
    data_map: BTreeMap<StateKey, Option<Vec<u8>>>,
}

impl<'a, S: StateView> StateViewCache<'a, S> {
    /// Create a `StateViewCache` give a `StateView`. Hold updates to the data store and
    /// forward data request to the `StateView` if not in the local cache.
    pub fn new(data_view: &'a S) -> Self {
        StateViewCache {
            data_view,
            data_map: BTreeMap::new(),
        }
    }

    // Publishes a `WriteSet` computed at the end of a transaction.
    // The effect is to build a layer in front of the `StateView` which keeps
    // track of the data as if the changes were applied immediately.
    pub(crate) fn push_write_set(&mut self, write_set: &WriteSet) {
        for (ap, write_op) in write_set.iter() {
            match write_op {
                WriteOp::Modification(blob) | WriteOp::Creation(blob) => {
                    self.data_map.insert(ap.clone(), Some(blob.clone()));
                }
                WriteOp::Deletion => {
                    self.data_map.remove(ap);
                    self.data_map.insert(ap.clone(), None);
                }
            }
        }
    }
}

impl<'block, S: StateView> StateView for StateViewCache<'block, S> {
    // Get some data either through the cache or the `StateView` on a cache miss.
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
        fail_point!("move_adapter::data_cache::get", |_| Err(format_err!(
            "Injected failure in data_cache::get"
        )));

        match self.data_map.get(state_key) {
            Some(opt_data) => Ok(opt_data.clone()),
            None => match self.data_view.get_state_value(state_key) {
                Ok(remote_data) => Ok(remote_data),
                // TODO: should we forward some error info?
                Err(e) => {
                    // create an AdapterLogSchema from the `data_view` in scope. This log_context
                    // does not carry proper information about the specific transaction and
                    // context, but this error is related to the given `StateView` rather
                    // than the transaction.
                    // Also this API does not make it easy to plug in a context
                    let log_context = AdapterLogSchema::new(self.data_view.id(), 0);
                    CRITICAL_ERRORS.inc();
                    error!(
                        log_context,
                        "[VM, StateView] Error getting data from storage for {:?}", state_key
                    );
                    Err(e)
                }
            },
        }
    }

    fn is_genesis(&self) -> bool {
        self.data_view.is_genesis()
    }

    fn id(&self) -> StateViewId {
        self.data_view.id()
    }

    fn get_usage(&self) -> Result<StateStorageUsage, Error> {
        self.data_view.get_usage()
    }
}

// Adapter to convert a `StateView` into a `RemoteCache`.
pub struct StorageAdapter<'a, S>(&'a S);

impl<'a, S: StateView> StorageAdapter<'a, S> {
    pub fn new(state_store: &'a S) -> Self {
        Self(state_store)
    }

    pub fn get(&self, access_path: &AccessPath) -> PartialVMResult<Option<Vec<u8>>> {
        self.0
            .get_state_value(&StateKey::AccessPath(access_path.clone()))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }
}

impl<'a, S: StateView> ModuleResolver for StorageAdapter<'a, S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        // REVIEW: cache this?
        let ap = AccessPath::from(module_id);
        self.get(&ap).map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, S: StateView> ResourceResolver for StorageAdapter<'a, S> {
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

impl<'a, S: StateView> TableResolver for StorageAdapter<'a, S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_state_value(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateView> ConfigStorage for StorageAdapter<'a, S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.get(&access_path).ok()?
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

impl<S: StateView> ModuleResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_module(module_id)
    }
}

impl<S: StateView> ResourceResolver for StorageAdapterOwned<S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.as_move_resolver().get_resource(address, struct_tag)
    }
}

impl<S: StateView> TableResolver for StorageAdapterOwned<S> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.as_move_resolver().resolve_table_entry(handle, key)
    }
}

impl<S: StateView> ConfigStorage for StorageAdapterOwned<S> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.as_move_resolver().fetch_config(access_path)
    }
}

impl<S: StateView> StateStorageUsageResolver for StorageAdapterOwned<S> {
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, anyhow::Error> {
        self.as_move_resolver().get_usage()
    }
}

pub trait IntoMoveResolver<S> {
    fn into_move_resolver(self) -> StorageAdapterOwned<S>;
}

impl<S: StateView> IntoMoveResolver<S> for S {
    fn into_move_resolver(self) -> StorageAdapterOwned<S> {
        StorageAdapterOwned { state_view: self }
    }
}
