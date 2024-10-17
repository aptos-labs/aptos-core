// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CacheRead,
    cross_block_caches::CrossBlockModuleCache,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_mvhashmap::types::VersionedModule;
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::ModuleCacheEntry,
};
use aptos_vm_types::module_and_script_storage::{
    code_storage::AptosCodeStorage, module_storage::AptosModuleStorage,
};
use bytes::Bytes;
use hashbrown::HashSet;
use move_binary_format::{
    errors::{Location, PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_runtime::{Module, ModuleStorage, RuntimeEnvironment, Script, WithRuntimeEnvironment};
use move_vm_types::{
    code::{Code, ModuleCache, ScriptCache},
    module_cyclic_dependency_error, module_linker_error,
};
use std::sync::Arc;

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> AptosCodeStorage
    for LatestView<'a, T, S, X>
{
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ScriptCache
    for LatestView<'a, T, S, X>
{
    type Deserialized = CompiledScript;
    type Key = [u8; 32];
    type Verified = Script;

    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized> {
        match &self.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .script_cache()
                .insert_deserialized_script(key, deserialized_script),
            ViewState::Unsync(state) => state
                .unsync_map
                .script_cache()
                .insert_deserialized_script(key, deserialized_script),
        }
    }

    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified> {
        match &self.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .script_cache()
                .insert_verified_script(key, verified_script),
            ViewState::Unsync(state) => state
                .unsync_map
                .script_cache()
                .insert_verified_script(key, verified_script),
        }
    }

    fn get_script(&self, key: &Self::Key) -> Option<Code<Self::Deserialized, Self::Verified>> {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.script_cache().get_script(key),
            ViewState::Unsync(state) => state.unsync_map.script_cache().get_script(key),
        }
    }

    fn num_scripts(&self) -> usize {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.script_cache().num_scripts(),
            ViewState::Unsync(state) => state.unsync_map.script_cache().num_scripts(),
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> AptosModuleStorage
    for LatestView<'a, T, S, X>
{
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        match CrossBlockModuleCache::fetch_state_value_metadata(address, module_name) {
            Some(state_value_metadata) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(state_value_metadata))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(read) = state
                        .captured_reads
                        .borrow()
                        .fetch_module_read(address, module_name)?
                    {
                        return Ok(read.map(|module| module.state_value_metadata().clone()));
                    }

                    let read = state
                        .read_module(address, module_name, &|id| self.get_base_module(id))
                        .map_err(|e| e.to_partial())?;
                    Ok(read.map(|v| v.state_value_metadata().clone()))
                },
                ViewState::Unsync(state) => {
                    let read = state
                        .read_module(address, module_name, &|id| self.get_base_module(id))
                        .map_err(|e| e.to_partial())?;
                    Ok(read.map(|e| e.state_value_metadata().clone()))
                },
            },
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ModuleStorage
    for LatestView<'a, T, S, X>
{
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        if CrossBlockModuleCache::check_module_exists(address, module_name) {
            self.capture_global_cache_read(address, module_name);
            return Ok(true);
        }

        Ok(self.read_module(address, module_name)?.is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        Ok(
            match CrossBlockModuleCache::fetch_module_bytes(address, module_name) {
                Some(bytes) => {
                    self.capture_global_cache_read(address, module_name);
                    Some(bytes)
                },
                None => self
                    .read_module(address, module_name)?
                    .map(|module| module.bytes().clone()),
            },
        )
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        Ok(
            match CrossBlockModuleCache::fetch_module_size_in_bytes(address, module_name) {
                Some(size) => {
                    self.capture_global_cache_read(address, module_name);
                    Some(size)
                },
                None => self
                    .read_module(address, module_name)?
                    .map(|module| module.size_in_bytes()),
            },
        )
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        Ok(
            match CrossBlockModuleCache::fetch_module_metadata(address, module_name) {
                Some(metadata) => {
                    self.capture_global_cache_read(address, module_name);
                    Some(metadata)
                },
                None => self
                    .read_module(address, module_name)?
                    .map(|module| module.metadata().to_vec()),
            },
        )
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        Ok(
            match CrossBlockModuleCache::fetch_deserialized_module(address, module_name) {
                Some(compiled_module) => {
                    self.capture_global_cache_read(address, module_name);
                    Some(compiled_module)
                },
                None => self
                    .read_module(address, module_name)?
                    .map(|module| module.compiled_module().clone()),
            },
        )
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        match CrossBlockModuleCache::fetch_verified_module(address, module_name)? {
            Some(module) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(module))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(read) = state
                        .captured_reads
                        .borrow()
                        .fetch_verified_module(address, module_name)?
                    {
                        return Ok(read);
                    }

                    unimplemented!()
                },
                ViewState::Unsync(state) => state.read_verified_module(
                    address,
                    module_name,
                    self.runtime_environment,
                    &|id| self.get_base_module(id),
                ),
            },
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> WithRuntimeEnvironment
    for LatestView<'a, T, S, X>
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.runtime_environment
    }
}

impl<'a, T: Transaction> SequentialState<'a, T> {
    /// Returns the module entry stored in the code cache, and if it is not there, initializes it.
    /// Also, records the read in the read-set.
    fn read_module<F>(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        init_func: &F,
    ) -> VMResult<Option<Arc<VersionedModule<ModuleCacheEntry>>>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<VersionedModule<ModuleCacheEntry>>>,
    {
        let module_id = ModuleId::new(*address, module_name.to_owned());
        let read = self
            .unsync_map
            .module_cache()
            .get_module_or_insert_with(&module_id, || init_func(&module_id))?;
        self.read_set.borrow_mut().capture_module_read(module_id);
        Ok(read)
    }

    /// Returns the verified module stored in the code cache, and if it is not there, initializes
    /// it. If the module has not been verified before, verifies it and its transitive dependencies
    /// (storing them to the module cache).
    fn read_verified_module<F>(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        runtime_environment: &RuntimeEnvironment,
        init_func: &F,
    ) -> VMResult<Option<Arc<Module>>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<VersionedModule<ModuleCacheEntry>>>,
    {
        // Check if module exists, recording the read as well. If it does not, return early.
        let entry = match self.read_module(address, module_name, &init_func)? {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // In case module is already verified, return early.
        if entry.is_verified() {
            return Ok(Some(entry.verified_module()?.clone()));
        }

        // Module exists, and is not verified. We need to verify it as well as load all transitive
        // dependencies.

        let mut visited = HashSet::new();
        let module_id = ModuleId::new(*address, module_name.to_owned());

        visited.insert(module_id.clone());
        let module = self.visit_dependencies_and_verify(
            entry.as_ref(),
            module_id,
            &mut visited,
            runtime_environment,
            init_func,
        )?;
        Ok(Some(module))
    }

    fn visit_dependencies_and_verify<F>(
        &self,
        entry: &VersionedModule<ModuleCacheEntry>,
        module_id: ModuleId,
        visited: &mut HashSet<ModuleId>,
        runtime_environment: &RuntimeEnvironment,
        init_func: &F,
    ) -> VMResult<Arc<Module>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<VersionedModule<ModuleCacheEntry>>>,
    {
        let compiled_module = entry.compiled_module();

        runtime_environment.paranoid_check_module_address_and_name(
            compiled_module,
            module_id.address(),
            module_id.name(),
        )?;
        let locally_verified_module = runtime_environment.build_locally_verified_module(
            compiled_module.clone(),
            entry.size_in_bytes(),
            entry.hash(),
        )?;

        let mut verified_dependencies = vec![];
        for (addr, name) in locally_verified_module.immediate_dependencies_iter() {
            let dependency = self
                .read_module(addr, name, init_func)?
                .ok_or_else(|| module_linker_error!(addr, name))?;

            if dependency.is_verified() {
                verified_dependencies.push(dependency.verified_module()?.clone());
                continue;
            }

            let dependency_module_id = ModuleId::new(*addr, name.to_owned());
            if visited.insert(dependency_module_id.clone()) {
                let verified_dependency = self.visit_dependencies_and_verify(
                    dependency.as_ref(),
                    dependency_module_id,
                    visited,
                    runtime_environment,
                    init_func,
                )?;
                verified_dependencies.push(verified_dependency);
            } else {
                return Err(module_cyclic_dependency_error!(
                    module_id.address(),
                    module_id.name()
                ));
            }
        }

        let module = runtime_environment
            .build_verified_module(locally_verified_module, &verified_dependencies)
            .map(Arc::new)?;

        let mm = entry.make_verified(module.clone());
        self.unsync_map
            .module_cache()
            .insert_module(module_id, VersionedModule::new(mm, entry.version()));

        Ok(module)
    }
}

impl<'a, T: Transaction, X: Executable> ParallelState<'a, T, X> {
    /// Returns the module stored in the code cache. If the module has not been cached, initializes
    /// it. The read is recorded in the read-set.
    fn read_module<F>(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        init_func: &F,
    ) -> VMResult<Option<Arc<VersionedModule<ModuleCacheEntry>>>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<VersionedModule<ModuleCacheEntry>>>,
    {
        let module_id = ModuleId::new(*address, module_name.to_owned());
        let read = self
            .versioned_map
            .module_cache()
            .get_module_or_insert_with(&module_id, || init_func(&module_id))?;
        self.captured_reads
            .borrow_mut()
            .capture_per_block_cache_read(module_id, read.clone());
        Ok(read)
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    /// Records the read from [CrossBlockModuleCache] in the read-set.
    fn capture_global_cache_read(&self, address: &AccountAddress, module_name: &IdentStr) {
        let module_id = ModuleId::new(*address, module_name.to_owned());
        match &self.latest_view {
            ViewState::Sync(state) => state
                .captured_reads
                .borrow_mut()
                .capture_global_cache_read(module_id),
            ViewState::Unsync(state) => state.read_set.borrow_mut().capture_module_read(module_id),
        }
    }

    fn read_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<VersionedModule<ModuleCacheEntry>>>> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                match state
                    .captured_reads
                    .borrow()
                    .fetch_module_read(address, module_name)?
                {
                    CacheRead::Hit(read) => Ok(read),
                    CacheRead::Miss => {
                        state.read_module(address, module_name, &|id| self.get_base_module(id))
                    },
                }
            },
            ViewState::Unsync(state) => {
                state.read_module(address, module_name, &|id| self.get_base_module(id))
            },
        }
    }

    /// Returns the module created from the pre-block state. The error is returned when the module
    /// creation fails (e.g., failed to deserialize bytes into the module), or when the underlying
    /// storage returns an error.
    fn get_base_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<VersionedModule<ModuleCacheEntry>>> {
        let key = T::Key::from_address_and_module_name(module_id.address(), module_id.name());
        self.get_raw_base_value(&key)
            .map_err(|err| err.finish(Location::Undefined))?
            .map(|state_value| {
                ModuleCacheEntry::from_state_value(self.runtime_environment, state_value)
                    .map(VersionedModule::from_pre_block_state)
            })
            .transpose()
    }
}
