// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_state_view::TStateView;
use aptos_types::{
    access_path::AccessPath,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
};
use aptos_vm_types::remote_cache::{TRemoteCache, TStateViewWithRemoteCache};
use move_core_types::language_storage::ModuleId;
use move_vm_types::resolver::{Module, ModuleRef, Resource, ResourceRef};
use std::collections::HashMap;

// `StateView` has no data given we are creating the genesis
pub(crate) struct GenesisStateView {
    state_data: HashMap<StateKey, Vec<u8>>,
}

impl GenesisStateView {
    pub(crate) fn new() -> Self {
        Self {
            state_data: HashMap::new(),
        }
    }

    pub(crate) fn add_module(&mut self, module_id: &ModuleId, blob: &[u8]) {
        self.state_data.insert(
            StateKey::access_path(AccessPath::from(module_id)),
            blob.to_vec(),
        );
    }
}

impl TStateView for GenesisStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        Ok(self
            .state_data
            .get(state_key)
            .cloned()
            .map(StateValue::new_legacy))
    }

    fn is_genesis(&self) -> bool {
        true
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::zero())
    }
}

impl TRemoteCache for GenesisStateView {
    type Key = StateKey;

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<ModuleRef>> {
        // TODO: Should we change `state_data` to ensure it is not stored as blobs?
        Ok(self
            .get_state_value_bytes(state_key)?
            .map(|blob| ModuleRef::new(Module::Serialized(blob))))
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<ResourceRef>> {
        // TODO: Should we change `state_data` to ensure it is not stored as blobs?
        Ok(self
            .get_state_value_bytes(state_key)?
            .map(|blob| ResourceRef::new(Resource::Serialized(blob))))
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> Result<Option<u128>> {
        // TODO: Should we change `state_data` to ensure it is not stored as blobs?
        Ok(match self.get_state_value_bytes(state_key)? {
            Some(blob) => Some(bcs::from_bytes(&blob)?),
            None => None,
        })
    }
}

impl TStateViewWithRemoteCache for GenesisStateView {
    type CommonKey = StateKey;
}
