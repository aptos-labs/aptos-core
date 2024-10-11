// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CacheRead,
    cross_block_caches::CrossBlockModuleCache,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_mvhashmap::code_cache::{LockedModuleCache, MaybeCommitted};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::{modules::ModuleCacheEntry, scripts::ScriptCacheEntry},
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
use move_vm_runtime::{
    compute_code_hash, logging::expect_no_verification_errors, CodeStorage, Module, ModuleStorage,
    RuntimeEnvironment, Script, WithRuntimeEnvironment,
};
use move_vm_types::{module_cyclic_dependency_error, module_linker_error, panic_error};
use std::sync::Arc;

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> AptosCodeStorage
    for LatestView<'a, T, S, X>
{
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> CodeStorage
    for LatestView<'a, T, S, X>
{
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        use ScriptCacheEntry::*;

        let hash = compute_code_hash(serialized_script);
        Ok(match self.fetch_script(&hash) {
            Some(script) => script.compiled_script().clone(),
            None => {
                let compiled_script = self
                    .runtime_environment
                    .deserialize_into_script(serialized_script)
                    .map(Arc::new)?;
                self.store_script(hash, Deserialized(compiled_script.clone()));
                compiled_script
            },
        })
    }

    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        use ScriptCacheEntry::*;

        let hash = compute_code_hash(serialized_script);
        let compiled_script = match self.fetch_script(&hash) {
            Some(Verified(script)) => return Ok(script),
            Some(Deserialized(compiled_script)) => compiled_script,
            None => self
                .runtime_environment
                .deserialize_into_script(serialized_script)
                .map(Arc::new)?,
        };

        // Locally verify the script.
        let locally_verified_script = self
            .runtime_environment
            .build_locally_verified_script(compiled_script)?;

        // Verify the script is correct w.r.t. its dependencies.
        let immediate_dependencies = locally_verified_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| {
                // Since module is stored on-chain, we should not see any verification errors here.
                self.fetch_verified_module(addr, name)
                    .map_err(expect_no_verification_errors)?
                    .ok_or_else(|| module_linker_error!(addr, name))
            })
            .collect::<VMResult<Vec<_>>>()?;
        let script = self
            .runtime_environment
            .build_verified_script(locally_verified_script, &immediate_dependencies)
            .map(Arc::new)?;

        self.store_script(hash, Verified(script.clone()));
        Ok(script)
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
                    if let CacheRead::Hit(result) = state
                        .captured_reads
                        .borrow()
                        .fetch_state_value_metadata(address, module_name)
                    {
                        return Ok(result);
                    }

                    state
                        .read_module_entry(
                            address,
                            module_name,
                            &|id| self.fetch_base_module_entry(id),
                            |r| r.map(|v| v.state_value_metadata().clone()),
                        )
                        .map_err(|e| e.to_partial())
                },
                ViewState::Unsync(state) => {
                    let read = state
                        .read_module_entry(address, module_name, &|id| {
                            self.fetch_base_module_entry(id)
                        })
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

        match &self.latest_view {
            ViewState::Sync(state) => {
                if let CacheRead::Hit(result) = state
                    .captured_reads
                    .borrow()
                    .check_module_exists(address, module_name)
                {
                    return Ok(result);
                }

                state.read_module_entry(
                    address,
                    module_name,
                    &|id| self.fetch_base_module_entry(id),
                    |r| r.is_some(),
                )
            },
            ViewState::Unsync(state) => Ok(state
                .read_module_entry(address, module_name, &|id| self.fetch_base_module_entry(id))?
                .is_some()),
        }
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        match CrossBlockModuleCache::fetch_module_bytes(address, module_name) {
            Some(bytes) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(bytes))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(maybe_bytes) = state
                        .captured_reads
                        .borrow()
                        .fetch_module_bytes(address, module_name)
                    {
                        return Ok(maybe_bytes);
                    }

                    state.read_module_entry(
                        address,
                        module_name,
                        &|id| self.fetch_base_module_entry(id),
                        |r| r.map(|v| v.bytes().clone()),
                    )
                },
                ViewState::Unsync(state) => {
                    let read = state.read_module_entry(address, module_name, &|id| {
                        self.fetch_base_module_entry(id)
                    })?;
                    Ok(read.map(|v| v.bytes().clone()))
                },
            },
        }
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        match CrossBlockModuleCache::fetch_module_size_in_bytes(address, module_name) {
            Some(size) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(size))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(maybe_size) = state
                        .captured_reads
                        .borrow()
                        .fetch_module_size_in_bytes(address, module_name)
                    {
                        return Ok(maybe_size);
                    }

                    state.read_module_entry(
                        address,
                        module_name,
                        &|id| self.fetch_base_module_entry(id),
                        |r| r.map(|v| v.size_in_bytes()),
                    )
                },
                ViewState::Unsync(state) => {
                    let read = state.read_module_entry(address, module_name, &|id| {
                        self.fetch_base_module_entry(id)
                    })?;
                    Ok(read.map(|v| v.size_in_bytes()))
                },
            },
        }
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        match CrossBlockModuleCache::fetch_module_metadata(address, module_name) {
            Some(metadata) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(metadata))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(maybe_metadata) = state
                        .captured_reads
                        .borrow()
                        .fetch_module_metadata(address, module_name)
                    {
                        return Ok(maybe_metadata);
                    }

                    state.read_module_entry(
                        address,
                        module_name,
                        &|id| self.fetch_base_module_entry(id),
                        |r| r.map(|v| v.metadata().to_vec()),
                    )
                },
                ViewState::Unsync(state) => {
                    let read = state.read_module_entry(address, module_name, &|id| {
                        self.fetch_base_module_entry(id)
                    })?;
                    Ok(read.map(|v| v.metadata().to_vec()))
                },
            },
        }
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        match CrossBlockModuleCache::fetch_deserialized_module(address, module_name) {
            Some(compiled_module) => {
                self.capture_global_cache_read(address, module_name);
                Ok(Some(compiled_module))
            },
            None => match &self.latest_view {
                ViewState::Sync(state) => {
                    if let CacheRead::Hit(maybe_compiled_module) = state
                        .captured_reads
                        .borrow()
                        .fetch_deserialized_module(address, module_name)
                    {
                        return Ok(maybe_compiled_module);
                    }

                    state.read_module_entry(
                        address,
                        module_name,
                        &|id| self.fetch_base_module_entry(id),
                        |r| r.map(|v| v.compiled_module().clone()),
                    )
                },
                ViewState::Unsync(state) => {
                    let read = state.read_module_entry(address, module_name, &|id| {
                        self.fetch_base_module_entry(id)
                    })?;
                    Ok(read.map(|v| v.compiled_module().clone()))
                },
            },
        }
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
            None => {
                match &self.latest_view {
                    ViewState::Sync(state) => {
                        if let CacheRead::Hit(result) = state
                            .captured_reads
                            .borrow()
                            .fetch_verified_module(address, module_name)
                        {
                            return result;
                        }

                        // This transaction has not read this module before, or the entry has not
                        // yet been verified. For verification, lock the cache.
                        let mut module_cache =
                            state.versioned_map.code_cache().module_cache().lock();

                        // The entry may not exist, in which case return early.
                        let module_id = ModuleId::new(*address, module_name.to_owned());
                        let entry = match module_cache.fetch_or_initialize(&module_id, &|id| {
                            self.fetch_base_module_entry(id)
                        })? {
                            Some(entry) => entry,
                            None => {
                                state
                                    .captured_reads
                                    .borrow_mut()
                                    .capture_block_cache_read(module_id, None);
                                module_cache.unlock();
                                return Ok(None);
                            },
                        };

                        // Also return early if the entry has already been verified.
                        if entry.is_verified() {
                            // We do not need to capture this read, because the other thread that
                            // verified this module has put it there.
                            // TODO(loader_v2): Add a paranoid check here.
                            module_cache.unlock();
                            return Ok(Some(entry.verified_module()?.clone()));
                        }

                        // Otherwise, we need to verify and load the transitive closure.

                        let mut visited = HashSet::new();
                        visited.insert(module_id.clone());

                        let module = ParallelState::<T, X>::visit_dependencies_and_verify(
                            entry.clone(),
                            &module_id,
                            &mut visited,
                            self.runtime_environment,
                            &mut module_cache,
                            &|id| self.fetch_base_module_entry(id),
                        )?;

                        let verified_entry = module_cache.fetch_or_initialize(&module_id, &|id| self.fetch_base_module_entry(id))?.ok_or_else(|| {
                            let msg = format!("Verified module {}::{} should be cached after, dependency traversal", module_id.address(), module_id.name());
                            panic_error!(msg).finish(Location::Undefined)
                        })?;

                        // TODO(loader_2): paranoid check same commit index?
                        state
                            .captured_reads
                            .borrow_mut()
                            .capture_block_cache_read(module_id, Some(verified_entry));

                        module_cache.unlock();
                        Ok(Some(module))
                    },
                    ViewState::Unsync(state) => state.read_verified_module(
                        address,
                        module_name,
                        self.runtime_environment,
                        &|id| self.fetch_base_module_entry(id),
                    ),
                }
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
    fn read_module_entry<F>(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        init_func: &F,
    ) -> VMResult<Option<Arc<ModuleCacheEntry>>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<ModuleCacheEntry>>,
    {
        let module_id = ModuleId::new(*address, module_name.to_owned());
        let read = self
            .unsync_map
            .code_cache()
            .fetch_or_initialize_module(&module_id, init_func)?;
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
        F: Fn(&ModuleId) -> VMResult<Option<ModuleCacheEntry>>,
    {
        // Check if module exists, recording the read as well. If it does not, return early.
        let entry = match self.read_module_entry(address, module_name, &init_func)? {
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
        entry: &ModuleCacheEntry,
        module_id: ModuleId,
        visited: &mut HashSet<ModuleId>,
        runtime_environment: &RuntimeEnvironment,
        init_func: &F,
    ) -> VMResult<Arc<Module>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<ModuleCacheEntry>>,
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
                .read_module_entry(addr, name, init_func)?
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

        let entry = entry.make_verified(module.clone());
        self.unsync_map.code_cache().store_module(module_id, entry);

        Ok(module)
    }
}

impl<'a, T: Transaction, X: Executable> ParallelState<'a, T, X> {
    /// Returns the specified information from the module stored in the code cache. If the module
    /// has not been cached, initializes it. The read is recorded in the read-set.
    fn read_module_entry<F, R, V>(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        init_func: &F,
        read_func: R,
    ) -> VMResult<V>
    where
        R: Fn(Option<&Arc<MaybeCommitted<ModuleCacheEntry>>>) -> V,
        F: Fn(&ModuleId) -> VMResult<Option<ModuleCacheEntry>>,
    {
        let module_id = ModuleId::new(*address, module_name.to_owned());
        let read = self
            .versioned_map
            .code_cache()
            .module_cache()
            .fetch_or_initialize(&module_id, init_func)?;

        let value = read_func(read.as_ref());
        self.captured_reads
            .borrow_mut()
            .capture_block_cache_read(module_id, read);
        Ok(value)
    }

    fn visit_dependencies_and_verify<F>(
        entry: Arc<MaybeCommitted<ModuleCacheEntry>>,
        module_id: &ModuleId,
        visited: &mut HashSet<ModuleId>,
        runtime_environment: &RuntimeEnvironment,
        locked_module_cache: &mut LockedModuleCache<ModuleId, ModuleCacheEntry>,
        init_func: &F,
    ) -> VMResult<Arc<Module>>
    where
        F: Fn(&ModuleId) -> VMResult<Option<ModuleCacheEntry>>,
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
            let dependency_module_id = ModuleId::new(*addr, name.to_owned());

            let dependency = locked_module_cache
                .fetch_or_initialize(&dependency_module_id, init_func)?
                .ok_or_else(|| module_linker_error!(addr, name))?;
            if dependency.is_verified() {
                verified_dependencies.push(dependency.verified_module()?.clone());
                continue;
            }

            if visited.insert(dependency_module_id.clone()) {
                let verified_dependency = Self::visit_dependencies_and_verify(
                    dependency,
                    &dependency_module_id,
                    visited,
                    runtime_environment,
                    locked_module_cache,
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
        let new_entry = entry.make_verified(module.clone());
        let entry = Arc::new(MaybeCommitted::verified(new_entry, entry.commit_idx()));
        locked_module_cache.store(module_id, entry)?;

        Ok(module)
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    /// Records the read from [CrossBlockModuleCache] in the read-set,
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

    /// Returns the module entry created from the base (storage) view. The error is returned if
    /// creation fails or storage returns an error.
    fn fetch_base_module_entry(&self, module_id: &ModuleId) -> VMResult<Option<ModuleCacheEntry>> {
        let key = T::Key::from_address_and_module_name(module_id.address(), module_id.name());
        self.get_raw_base_value(&key)
            .map_err(|e| e.finish(Location::Undefined))?
            .map(|s| ModuleCacheEntry::from_state_value(self.runtime_environment, s))
            .transpose()
    }

    /// Returns the script stored in the code cache. If it does not exist, returns [None].
    fn fetch_script(&self, hash: &[u8; 32]) -> Option<ScriptCacheEntry> {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.code_cache().fetch_script(hash),
            ViewState::Unsync(state) => state.unsync_map.code_cache().fetch_script(hash),
        }
    }

    /// Adds the script to the code cache.
    fn store_script(&self, hash: [u8; 32], script: ScriptCacheEntry) {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.code_cache().store_script(hash, script),
            ViewState::Unsync(state) => state.unsync_map.code_cache().store_script(hash, script),
        }
    }
}
