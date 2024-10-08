// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::FETCH_NOT_CACHED_VERIFIED_MODULE_SECONDS,
    cross_block_caches::CrossBlockModuleCache,
    view::{LatestView, ViewState},
};
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
use move_vm_types::{module_cyclic_dependency_error, module_linker_error};
use std::{collections::HashSet, sync::Arc};

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> AptosCodeStorage
    for LatestView<'a, T, S, X>
{
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> AptosModuleStorage
    for LatestView<'a, T, S, X>
{
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        if let Some(state_value_metadata) =
            CrossBlockModuleCache::fetch_module_state_value_metadata(address, module_name)
        {
            return Ok(Some(state_value_metadata));
        }

        Ok(self
            .read_module_storage(address, module_name)
            .map_err(|e| e.to_partial())?
            .map(|entry| entry.state_value_metadata().clone()))
    }
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
        let entry = match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.code_cache().fetch_cached_script(&hash),
            ViewState::Unsync(state) => state.unsync_map.code_cache().fetch_cached_script(&hash),
        };

        Ok(match entry {
            Some(Verified(script)) => script.as_compiled_script(),
            Some(Deserialized(compiled_script)) => compiled_script,
            None => {
                let compiled_script = Arc::new(
                    self.runtime_environment
                        .deserialize_into_script(serialized_script)?,
                );

                let entry = Deserialized(compiled_script.clone());
                match &self.latest_view {
                    ViewState::Sync(state) => {
                        state.versioned_map.code_cache().cache_script(hash, entry)
                    },
                    ViewState::Unsync(state) => {
                        state.unsync_map.code_cache().cache_script(hash, entry)
                    },
                }
                compiled_script
            },
        })
    }

    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        use ScriptCacheEntry::*;

        let hash = compute_code_hash(serialized_script);
        let entry = match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.code_cache().fetch_cached_script(&hash),
            ViewState::Unsync(state) => state.unsync_map.code_cache().fetch_cached_script(&hash),
        };

        let compiled_script = match entry {
            Some(Verified(script)) => return Ok(script),
            Some(Deserialized(compiled_script)) => compiled_script,
            None => self.deserialize_and_cache_script(serialized_script)?,
        };

        // Locally verify the script.
        let locally_verified_script = self
            .runtime_environment
            .build_locally_verified_script(compiled_script)?;

        // Verify the script by also looking at its dependencies.
        let immediate_dependencies = locally_verified_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| {
                self.fetch_verified_module(addr, name)
                    .map_err(expect_no_verification_errors)?
                    .ok_or_else(|| module_linker_error!(addr, name))
            })
            .collect::<VMResult<Vec<_>>>()?;
        let script = self
            .runtime_environment
            .build_verified_script(locally_verified_script, &immediate_dependencies)?;
        let script = Arc::new(script);

        let entry = Verified(script.clone());
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.code_cache().cache_script(hash, entry),
            ViewState::Unsync(state) => state.unsync_map.code_cache().cache_script(hash, entry),
        }
        Ok(script)
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
            return Ok(true);
        }

        Ok(self.read_module_storage(address, module_name)?.is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        if let Some(bytes) = CrossBlockModuleCache::fetch_module_bytes(address, module_name) {
            return Ok(Some(bytes));
        }

        Ok(self
            .read_module_storage(address, module_name)?
            .map(|entry| entry.bytes().clone()))
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        if let Some(size) = CrossBlockModuleCache::fetch_module_size_in_bytes(address, module_name)
        {
            return Ok(Some(size));
        }

        Ok(self
            .read_module_storage(address, module_name)?
            .map(|entry| entry.bytes().len()))
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        if let Some(metadata) = CrossBlockModuleCache::fetch_module_metadata(address, module_name) {
            return Ok(Some(metadata));
        }

        Ok(self
            .read_module_storage(address, module_name)?
            .map(|entry| entry.metadata().to_vec()))
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        if let Some(compiled_module) =
            CrossBlockModuleCache::fetch_deserialized_module(address, module_name)
        {
            return Ok(Some(compiled_module));
        }

        Ok(self
            .read_module_storage(address, module_name)?
            .map(|entry| entry.as_compiled_module()))
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        if let Some(module) = CrossBlockModuleCache::fetch_verified_module(address, module_name) {
            return Ok(Some(module));
        }

        let _timer = FETCH_NOT_CACHED_VERIFIED_MODULE_SECONDS.start_timer();

        let entry = match self.read_module_storage(address, module_name)? {
            Some(entry) => entry,
            None => return Ok(None),
        };
        if let Some(module) = entry.try_as_verified_module() {
            return Ok(Some(module));
        }

        let mut visited = HashSet::new();
        let module =
            self.traversed_published_dependencies(entry, address, module_name, &mut visited)?;
        Ok(Some(module))
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> WithRuntimeEnvironment
    for LatestView<'a, T, S, X>
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.runtime_environment
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    /// Returns the module storage entry built from the base (storage) view. If there is a
    /// storage error, returns an error.
    fn get_base_module_storage_entry(
        &self,
        key: &T::Key,
    ) -> VMResult<Option<Arc<ModuleCacheEntry>>> {
        self.get_raw_base_value(key)
            .map_err(|e| e.finish(Location::Undefined))?
            .map(|s| ModuleCacheEntry::from_state_value(self.runtime_environment, s).map(Arc::new))
            .transpose()
    }

    /// Returns the module storage entry built from the current view. If it is not in
    /// multi-version or non-sync data structures, fetches it from the base view.
    fn read_module_storage(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<ModuleCacheEntry>>> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                // If the module read has been previously cached, return early.
                if let Some(entry) = state
                    .captured_reads
                    .borrow()
                    .get_captured_module_read(address, module_name)
                {
                    return Ok(entry.clone());
                }

                // Otherwise, we need to go to the module cache to get it, and
                // record under captured reads.
                let module_id = ModuleId::new(*address, module_name.to_owned());
                let read = state
                    .versioned_map
                    .code_cache()
                    .module_cache()
                    .fetch_cached_module_or_initialize(&module_id, || {
                        let key = T::Key::from_address_and_module_name(address, module_name);
                        self.get_base_module_storage_entry(&key)
                    })?;
                state
                    .captured_reads
                    .borrow_mut()
                    .capture_module_read(module_id, read.clone());
                Ok(read)
            },
            ViewState::Unsync(state) => {
                state
                    .read_set
                    .borrow_mut()
                    .module_storage_reads
                    .insert(ModuleId::new(*address, module_name.to_owned()));

                match state
                    .unsync_map
                    .code_cache()
                    .fetch_cached_module(address, module_name)
                {
                    // For sequential execution, indices do not matter, but we still return
                    // them to have uniform interfaces.
                    Some(entry) => Ok(Some(entry)),
                    None => {
                        let key = T::Key::from_address_and_module_name(address, module_name);
                        self.get_base_module_storage_entry(&key)
                    },
                }
            },
        }
    }

    /// Similar to [LatestView::read_module_storage], but in case the module does not exist,
    /// returns a linker VM error.
    fn get_existing_module_storage_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<ModuleCacheEntry>> {
        self.read_module_storage(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Given module's address and name, returns its verified representation. In case it is
    /// not verified, verifies all its unverified transitive dependencies. As a side effect,
    /// the verified dependencies are made visible in the module storage.
    fn traversed_published_dependencies(
        &self,
        entry: Arc<ModuleCacheEntry>,
        address: &AccountAddress,
        module_name: &IdentStr,
        visited: &mut HashSet<ModuleId>,
    ) -> VMResult<Arc<Module>> {
        // At this point, the following holds:
        //  1. The version of the returned entry corresponds to a committed transaction. This is
        //     true because otherwise we would observe non-existent module and exit early.
        //  2. Entry exists at this index.

        // Otherwise, run the local verification first.
        let cm = entry.as_compiled_module();
        self.runtime_environment
            .paranoid_check_module_address_and_name(cm.as_ref(), address, module_name)?;

        let size = entry.size_in_bytes();
        let hash = entry.hash();
        let locally_verified_module = self
            .runtime_environment
            .build_locally_verified_module(cm, size, hash)?;

        // Next, before we complete the verification by checking immediate dependencies, we need
        // to make sure all of them are also verified.
        let mut verified_dependencies = vec![];
        for (addr, name) in locally_verified_module.immediate_dependencies_iter() {
            // A verified dependency, continue early.
            let dep_entry = self.get_existing_module_storage_entry(addr, name)?;
            if let Some(module) = dep_entry.try_as_verified_module() {
                verified_dependencies.push(module);
                continue;
            }

            // Otherwise the dependency has not been verified. If the currently executed thread
            // did not see it, we simply recurse to verify at as well. Otherwise, the thread must
            // have seen it before but have not yet verified (the check above ensures that). Hence,
            // there must be a cycle.
            // Note: here we treat "verified" modules as graph nodes that exited the recursion,
            //       which allows us to identify cycles.
            assert!(!dep_entry.is_verified());

            if visited.insert(ModuleId::new(*addr, name.to_owned())) {
                let module =
                    self.traversed_published_dependencies(dep_entry, addr, name, visited)?;
                verified_dependencies.push(module);
            } else {
                return Err(module_cyclic_dependency_error!(address, module_name));
            }
        }

        // At this point, all dependencies of the module are verified, so we can run final checks
        // and construct a verified module.
        let module = Arc::new(
            self.runtime_environment
                .build_verified_module(locally_verified_module, &verified_dependencies)?,
        );
        let verified_entry = Arc::new(entry.make_verified(module.clone()));

        // Finally, change the entry in the module storage to the verified one, in order to
        // make sure that everyone sees the verified module.
        let id = ModuleId::new(*address, module_name.to_owned());
        match &self.latest_view {
            ViewState::Sync(state) => {
                state
                    .captured_reads
                    .borrow_mut()
                    .capture_module_read(id.clone(), Some(verified_entry.clone()));
                state
                    .versioned_map
                    .code_cache()
                    .module_cache()
                    .cache_module(id, verified_entry);
            },
            ViewState::Unsync(state) => {
                state
                    .unsync_map
                    .code_cache()
                    .cache_module(id, verified_entry);
            },
        }
        Ok(module)
    }
}
