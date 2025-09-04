// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_types::{
    executable::ModulePath,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, TStateView,
    },
    write_set::WriteOp,
};
use velor_vm_types::module_write_set::ModuleWrite;
use bytes::Bytes;
use claims::assert_some;
use move_core_types::language_storage::ModuleId;
use std::collections::{BTreeMap, HashMap};

type Result<T, E = StateViewError> = std::result::Result<T, E>;

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
        self.state_data
            .insert(StateKey::module_id(module_id), blob.to_vec());
    }

    pub(crate) fn add_module_write_ops(
        &mut self,
        module_write_ops: BTreeMap<StateKey, ModuleWrite<WriteOp>>,
    ) {
        for (state_key, write) in module_write_ops {
            assert!(state_key.is_module_path());
            let bytes = assert_some!(write.write_op().bytes(), "Modules cannot be deleted");
            self.state_data.insert(state_key, bytes.to_vec());
        }
    }
}

impl TStateView for GenesisStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        Ok(self
            .state_data
            .get(state_key)
            .map(|bytes| StateValue::new_legacy(Bytes::copy_from_slice(bytes))))
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::zero())
    }
}
