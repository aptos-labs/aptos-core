// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::view::{LatestView, ViewState};
use aptos_mvhashmap::{types::ShiftedTxnIndex, versioned_module_storage::ModuleStorageReadResult};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{state_key::StateKey, state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::ModuleStorageEntry,
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
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_runtime::{
    deserialize_script, logging::expect_no_verification_errors, module_cyclic_dependency_error,
    module_linker_error, script_hash, CodeStorage, Module, ModuleStorage, Script,
};
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
        Ok(self
            .get_module_storage_entry(address, module_name)
            .map_err(|e| e.to_partial())?
            .into_module_module_storage_entry_at_idx()
            .map(|(_, entry)| entry.state_value_metadata().clone()))
    }

    fn fetch_module_size_by_state_key(
        &self,
        state_key: &StateKey,
    ) -> PartialVMResult<Option<usize>> {
        // TODO(loader_v2): A very ugly way of converting state keys into generic types.
        let key = T::Key::from_state_key(state_key.clone());
        Ok(self
            .get_module_storage_entry_by_key(&key)
            .map_err(|e| e.to_partial())?
            .into_module_module_storage_entry_at_idx()
            .map(|(_, entry)| entry.bytes().len()))
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> CodeStorage
    for LatestView<'a, T, S, X>
{
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        let hash = script_hash(serialized_script);

        let maybe_compiled_script = match &self.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .code_storage()
                .get_deserialized_script(&hash),
            ViewState::Unsync(state) => state.unsync_map.get_deserialized_script(&hash),
        };

        Ok(match maybe_compiled_script {
            Some(compiled_script) => compiled_script,
            None => {
                let compiled_script = self.deserialize_script(serialized_script)?;

                match &self.latest_view {
                    ViewState::Sync(state) => state
                        .versioned_map
                        .code_storage()
                        .cache_deserialized_script(hash, compiled_script.clone()),
                    ViewState::Unsync(state) => state
                        .unsync_map
                        .cache_deserialized_script(hash, compiled_script.clone()),
                }
                compiled_script
            },
        })
    }

    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        let hash = script_hash(serialized_script);

        let maybe_verified_script = match &self.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .code_storage()
                .get_verified_script(&hash),
            ViewState::Unsync(state) => state.unsync_map.get_verified_script(&hash),
        };

        let compiled_script = match maybe_verified_script {
            Some(Ok(script)) => return Ok(script),
            Some(Err(compiled_script)) => compiled_script,
            None => self.deserialize_and_cache_script(serialized_script)?,
        };

        // Locally verify the script.
        let partially_verified_script = self
            .runtime_environment
            .build_partially_verified_script(compiled_script)?;

        // Verify the script by also looking at its dependencies.
        let immediate_dependencies = partially_verified_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| {
                self.fetch_verified_module(addr, name)
                    .map_err(expect_no_verification_errors)
            })
            .collect::<VMResult<Vec<_>>>()?;
        let script = self
            .runtime_environment
            .build_verified_script(partially_verified_script, &immediate_dependencies)?;
        let script = Arc::new(script);

        match &self.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .code_storage()
                .cache_verified_script(hash, script.clone()),
            ViewState::Unsync(state) => {
                state.unsync_map.cache_verified_script(hash, script.clone())
            },
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
        Ok(self
            .get_module_storage_entry(address, module_name)?
            .into_module_module_storage_entry_at_idx()
            .is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        Ok(self
            .get_module_storage_entry(address, module_name)?
            .into_module_module_storage_entry_at_idx()
            .map(|(_, entry)| entry.bytes().clone()))
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        Ok(self
            .get_module_storage_entry(address, module_name)?
            .into_module_module_storage_entry_at_idx()
            .map(|(_, entry)| entry.bytes().len()))
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        Ok(self
            .get_existing_module_storage_entry_with_idx(address, module_name)?
            .1
            .metadata()
            .to_vec())
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        Ok(self
            .get_existing_module_storage_entry_with_idx(address, module_name)?
            .1
            .as_compiled_module())
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        let mut visited = HashSet::new();
        self.traversed_published_dependencies(address, module_name, &mut visited)
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    /// Returns the module storage entry built from the base (storage) view. If there is a
    /// storage error, returns an error.
    fn get_base_module_storage_entry(
        &self,
        key: &T::Key,
    ) -> VMResult<Option<Arc<ModuleStorageEntry>>> {
        self.get_raw_base_value(key)
            .map_err(|e| e.finish(Location::Undefined))?
            .map(|s| {
                ModuleStorageEntry::from_state_value(self.runtime_environment, s).map(Arc::new)
            })
            .transpose()
    }

    /// Returns the module storage entry built from the current view. If it is not in
    /// multi-version or non-sync data structures, fetches it from the base view.
    fn get_module_storage_entry_by_key(&self, key: &T::Key) -> VMResult<ModuleStorageReadResult> {
        use ModuleStorageReadResult::*;

        match &self.latest_view {
            ViewState::Sync(state) => {
                let result = state
                    .versioned_map
                    .code_storage()
                    .module_storage()
                    .get_or_else(key, ShiftedTxnIndex::new(self.txn_idx), || {
                        self.get_base_module_storage_entry(key)
                    })?;
                state
                    .captured_reads
                    .borrow_mut()
                    .capture_module_storage_read(key.clone(), &result);
                Ok(result)
            },
            ViewState::Unsync(state) => {
                state.read_set.borrow_mut().module_reads.insert(key.clone());
                Ok(match state.unsync_map.fetch_module(key) {
                    // For sequential execution, indices do not matter, but we still return
                    // them to have uniform interfaces.
                    Some(entry) => Versioned(ShiftedTxnIndex::previous_idx(self.txn_idx), entry),
                    None => match self.get_base_module_storage_entry(key)? {
                        Some(entry) => Versioned(ShiftedTxnIndex::zero_idx(), entry),
                        None => DoesNotExist,
                    },
                })
            },
        }
    }

    /// Similar to [LatestView::get_module_storage_entry_by_key], but allows to resolve module
    /// storage entries based on addresses and names.
    fn get_module_storage_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<ModuleStorageReadResult> {
        let key = T::Key::from_address_and_module_name(address, module_name);
        self.get_module_storage_entry_by_key(&key)
    }

    /// Similar to [LatestView::get_module_storage_entry], but in case the module does not exist,
    /// returns a linker VM error.
    fn get_existing_module_storage_entry_with_idx(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<(ShiftedTxnIndex, Arc<ModuleStorageEntry>)> {
        self.get_module_storage_entry(address, module_name)?
            .into_module_module_storage_entry_at_idx()
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Given module's address and name, returns its verified representation. In case it is
    /// not verified, verifies all its unverified transitive dependencies. As a side effect,
    /// the verified dependencies are made visible in the module storage.
    fn traversed_published_dependencies(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        visited: &mut HashSet<T::Key>,
    ) -> VMResult<Arc<Module>> {
        // Get the module and check if is verified, if so, return it.
        let (txn_idx, entry) =
            self.get_existing_module_storage_entry_with_idx(address, module_name)?;
        if let Some(module) = entry.try_as_verified_module() {
            return Ok(module);
        }

        // At this point, the following holds:
        //  1. The index of returned entry corresponds to a committed transaction.
        //     This is true because otherwise we would observe non-existent module
        //     an exit early.
        //  2. Entry exists at this index.

        // Otherwise, run the local verification first.
        let size = entry.bytes().len();
        let cm = entry.as_compiled_module();
        self.runtime_environment
            .paranoid_check_module_address_and_name(cm.as_ref(), address, module_name)?;
        let partially_verified_module = self
            .runtime_environment
            .build_partially_verified_module(cm, size)?;

        // Next, before we complete the verification by checking immediate dependencies, we need
        // to make sure all of them are also verified.
        let mut verified_dependencies = vec![];
        for (addr, name) in partially_verified_module.immediate_dependencies_iter() {
            // A verified dependency, continue early.
            let (_, dep_entry) = self.get_existing_module_storage_entry_with_idx(addr, name)?;
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
            let dep_key = T::Key::from_address_and_module_name(addr, name);
            if visited.insert(dep_key.clone()) {
                let module = self.traversed_published_dependencies(addr, name, visited)?;
                verified_dependencies.push(module);
            } else {
                return Err(module_cyclic_dependency_error!(address, module_name));
            }
        }

        // At this point, all dependencies of the module are verified, so we can run final checks
        // and construct a verified module.
        let module = Arc::new(
            self.runtime_environment
                .build_verified_module(partially_verified_module, &verified_dependencies)?,
        );
        let verified_entry = entry.make_verified(module.clone());

        // Finally, change the entry in the module storage to the verified one, in order to
        // make sure that everyone sees the verified module.
        let key = T::Key::from_address_and_module_name(address, module_name);
        match &self.latest_view {
            ViewState::Sync(state) => {
                state
                    .versioned_map
                    .code_storage()
                    .module_storage()
                    .write_if_not_verified(&key, txn_idx, verified_entry);
            },
            ViewState::Unsync(state) => {
                state
                    .unsync_map
                    .write_module_storage_entry(key, verified_entry);
            },
        }
        Ok(module)
    }

    /// Returns the deserialized script based on the current runtime environment.
    fn deserialize_script(&self, serialized_script: &[u8]) -> VMResult<Arc<CompiledScript>> {
        let deserializer_config = &self.runtime_environment.vm_config().deserializer_config;
        let compiled_script = deserialize_script(serialized_script, deserializer_config)?;
        Ok(Arc::new(compiled_script))
    }
}
