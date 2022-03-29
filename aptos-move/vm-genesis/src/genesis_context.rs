// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::{access_path::AccessPath, state_store::state_key::StateKey};
use move_core_types::language_storage::ModuleId;
use std::collections::HashMap;

// `StateView` has no data given we are creating the genesis
pub(crate) struct GenesisStateView {
    account_data: HashMap<AccessPath, Vec<u8>>,
    state_data: HashMap<StateKey, Vec<u8>>,
}

impl GenesisStateView {
    pub(crate) fn new() -> Self {
        Self {
            account_data: HashMap::new(),
            state_data: HashMap::new(),
        }
    }

    pub(crate) fn add_module(&mut self, module_id: &ModuleId, blob: &[u8]) {
        let access_path = AccessPath::from(module_id);
        self.account_data.insert(access_path, blob.to_vec());
    }
}

impl StateView for GenesisStateView {
    fn get_by_access_path(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>> {
        Ok(self.account_data.get(access_path).cloned())
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        Ok(self.state_data.get(state_key).cloned())
    }

    fn is_genesis(&self) -> bool {
        true
    }
}
