// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_cache::RemoteStorage;
use aptos_parallel_executor::executor::MVHashMapView;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::{access_path::AccessPath, state_store::state_key::StateKey, write_set::WriteOp};
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::{ModuleResolver, ResourceResolver},
};

pub(crate) struct VersionedView<'a, S: StateView> {
    base_view: &'a S,
    hashmap_view: &'a MVHashMapView<'a, AccessPath, WriteOp>,
}

impl<'a, S: StateView> VersionedView<'a, S> {
    pub fn new_view(
        base_view: &'a S,
        hashmap_view: &'a MVHashMapView<'a, AccessPath, WriteOp>,
    ) -> VersionedView<'a, S> {
        VersionedView {
            base_view,
            hashmap_view,
        }
    }
}

impl<'a, S: StateView> StateView for VersionedView<'a, S> {
    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    // Get some data either through the cache or the `StateView` on a cache miss.
    fn get_by_access_path(&self, access_path: &AccessPath) -> anyhow::Result<Option<Vec<u8>>> {
        match self.hashmap_view.read(access_path) {
            Ok(Some(v)) => Ok(match v.as_ref() {
                WriteOp::Value(w) => Some(w.clone()),
                WriteOp::Deletion => None,
            }),
            Ok(None) => self.base_view.get_by_access_path(access_path),
            Err(err) => Err(err),
        }
    }

    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
        // TODO: Add a caching layer on this once the VM write set starts populating state_value changes.
        self.base_view.get_state_value(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.base_view.is_genesis()
    }
}

impl<'a, S: StateView> ModuleResolver for VersionedView<'a, S> {
    type Error = VMError;

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        RemoteStorage::new(self).get_module(module_id)
    }
}

impl<'a, S: StateView> ResourceResolver for VersionedView<'a, S> {
    type Error = VMError;

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        RemoteStorage::new(self).get_resource(address, tag)
    }
}
