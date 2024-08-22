// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_write_set::ModuleWriteSet;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, metadata::Metadata,
    vm_status::StatusCode,
};
use move_vm_runtime::{Module, ModuleStorage, RuntimeEnvironment};
use std::{collections::BTreeMap, sync::Arc};

/// A module storage with stashed temporary changes (write ops). Use by AptosVM to
/// process init_module, where published modules are made temporarily visible.
pub struct TemporaryModuleStorage<'a, M> {
    runtime_env: &'a RuntimeEnvironment,

    // TODO(loader_v2): Implement cache which "caches" deserialization? Or we can construct
    //                  Arc<Module> and store it inside MVHashMap instead!
    write_ops: BTreeMap<StateKey, WriteOp>,
    module_storage: &'a M,
}

impl<'a, M: ModuleStorage> TemporaryModuleStorage<'a, M> {
    /// Creates a new temporary module storage with stashed changes.
    pub fn create(
        runtime_env: &'a RuntimeEnvironment,
        write_ops: BTreeMap<StateKey, WriteOp>,
        module_storage: &'a M,
    ) -> Self {
        Self {
            runtime_env,
            write_ops,
            module_storage,
        }
    }

    /// Destroys temporary module storage releasing the stshed changes.
    pub fn destroy(self) -> ModuleWriteSet {
        // We do not care here about writes to special addresses because there is no flushing.
        ModuleWriteSet::new(false, self.write_ops)
    }

    fn write_op_to_compiled_module(
        &self,
        write_op: &WriteOp,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        // Modules can never be deleted, return an invariant violation for extra safety.
        let bytes = write_op.bytes().ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Module deletion is not be possible".to_string())
        })?;

        let deserializer_config = &self.runtime_env.vm_config().deserializer_config;
        Ok(Arc::new(CompiledModule::deserialize_with_config(
            bytes,
            deserializer_config,
        )?))
    }
}

impl<'a, M: ModuleStorage> ModuleStorage for TemporaryModuleStorage<'a, M> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        let state_key = StateKey::module(address, module_name);
        if self.write_ops.contains_key(&state_key) {
            return Ok(true);
        }
        self.module_storage
            .check_module_exists(address, module_name)
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        let state_key = StateKey::module(address, module_name);
        match self.write_ops.get(&state_key) {
            Some(write_op) => Ok(write_op.size()),
            None => self
                .module_storage
                .fetch_module_size_in_bytes(address, module_name),
        }
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        let deserialized_module = self.fetch_deserialized_module(address, module_name)?;
        Ok(deserialized_module.metadata.clone())
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        let state_key = StateKey::module(address, module_name);
        match self.write_ops.get(&state_key) {
            Some(write_op) => self.write_op_to_compiled_module(write_op),
            None => self
                .module_storage
                .fetch_deserialized_module(address, module_name),
        }
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<Module>> {
        let state_key = StateKey::module(address, module_name);
        match self.write_ops.get(&state_key) {
            Some(write_op) => {
                let compiled_module = self.write_op_to_compiled_module(write_op)?;
                let partially_verified_module = self
                    .runtime_env
                    .build_partially_verified_module(compiled_module)?;
                // TODO(loader_v2): Revisit this, technically by this point we have checked there are no cycles and all modules must be loaded.
                let immediate_dependencies = partially_verified_module
                    .immediate_dependencies_iter()
                    .map(|(addr, name)| self.fetch_verified_module(addr, name))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                let verified_module = self
                    .runtime_env
                    .build_verified_module(partially_verified_module, &immediate_dependencies)?;
                Ok(Arc::new(verified_module))
            },
            None => self
                .module_storage
                .fetch_verified_module(address, module_name),
        }
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2): Implement tests.
}
