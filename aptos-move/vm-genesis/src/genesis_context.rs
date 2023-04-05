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
use move_core_types::language_storage::ModuleId;
use std::collections::HashMap;
use aptos_vm_view::types::{AptosResource, TRemoteCache, TStateViewWithRemoteCache};

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

    fn get_cached_aggregator_value(&self, state_key: &Self::Key) -> Result<Option<u128>> {
        todo!()
    }

    fn get_cached_module(&self, state_key: &Self::Key) -> Result<Option<Vec<u8>>> {
        todo!()
    }

    fn get_cached_resource(&self, state_key: &Self::Key) -> Result<Option<AptosResource<Self::Key>>> {
        todo!()
    }
}

impl TStateViewWithRemoteCache for GenesisStateView { type CommonKey = StateKey; }
