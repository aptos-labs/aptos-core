// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::BlockSynchronizationKillSwitch,
};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use bytes::Bytes;
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use move_vm_runtime::{
    LayoutCache, LayoutCacheEntry, Module, ModuleStorage, RuntimeEnvironment, Script, StructKey,
    WithRuntimeEnvironment,
};
use move_vm_types::code::{Code, ScriptCache};
use std::{cell::RefCell, collections::HashSet, sync::Arc};

/// Wraps a code storage and records the state key of every module the VM fetches through it,
/// so that module accesses become part of the transaction's observed read set (the basis for
/// hot state promotion).
///
/// Scripts are not state items, so script cache accesses are not recorded.
pub struct ReadRecordingCodeStorage<'a, C> {
    code_storage: &'a C,
    module_reads: RefCell<HashSet<StateKey>>,
}

impl<'a, C> ReadRecordingCodeStorage<'a, C> {
    pub fn new(code_storage: &'a C) -> Self {
        Self {
            code_storage,
            module_reads: RefCell::new(HashSet::new()),
        }
    }

    /// Returns the state keys of modules fetched so far.
    pub fn into_recorded_reads(self) -> HashSet<StateKey> {
        self.module_reads.take()
    }

    fn record(&self, address: &AccountAddress, module_name: &IdentStr) {
        self.module_reads
            .borrow_mut()
            .insert(StateKey::module(address, module_name));
    }
}

impl<C: WithRuntimeEnvironment> WithRuntimeEnvironment for ReadRecordingCodeStorage<'_, C> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_storage.runtime_environment()
    }
}

impl<C: LayoutCache> LayoutCache for ReadRecordingCodeStorage<'_, C> {
    fn get_struct_layout(&self, key: &StructKey) -> Option<LayoutCacheEntry> {
        self.code_storage.get_struct_layout(key)
    }

    fn store_struct_layout(&self, key: &StructKey, entry: LayoutCacheEntry) -> PartialVMResult<()> {
        self.code_storage.store_struct_layout(key, entry)
    }

    fn remove_struct_layout(&self, key: &StructKey) {
        self.code_storage.remove_struct_layout(key)
    }
}

impl<C: ModuleStorage> ModuleStorage for ReadRecordingCodeStorage<'_, C> {
    fn unmetered_check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_check_module_exists(address, module_name)
    }

    fn unmetered_get_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_bytes(address, module_name)
    }

    fn unmetered_get_module_hash_and_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<([u8; 32], usize)>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_hash_and_size(address, module_name)
    }

    fn unmetered_get_module_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_size(address, module_name)
    }

    fn unmetered_get_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_deserialized_module(address, module_name)
    }

    fn unmetered_get_eagerly_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_eagerly_verified_module(address, module_name)
    }

    fn unmetered_get_lazily_verified_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<Module>>> {
        self.record(module_id.address(), module_id.name());
        self.code_storage
            .unmetered_get_lazily_verified_module(module_id)
    }

    #[cfg(fuzzing)]
    fn unmetered_get_module_skip_verification(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_skip_verification(address, module_name)
    }
}

impl<C: AptosModuleStorage> AptosModuleStorage for ReadRecordingCodeStorage<'_, C> {
    fn unmetered_get_module_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_state_value_metadata(address, module_name)
    }
}

impl<C> ScriptCache for ReadRecordingCodeStorage<'_, C>
where
    C: ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>,
{
    type Deserialized = CompiledScript;
    type Key = [u8; 32];
    type Verified = Script;

    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized> {
        self.code_storage
            .insert_deserialized_script(key, deserialized_script)
    }

    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified> {
        self.code_storage
            .insert_verified_script(key, verified_script)
    }

    fn get_script(&self, key: &Self::Key) -> Option<Code<Self::Deserialized, Self::Verified>> {
        self.code_storage.get_script(key)
    }

    fn num_scripts(&self) -> usize {
        self.code_storage.num_scripts()
    }
}

impl<C: BlockSynchronizationKillSwitch> BlockSynchronizationKillSwitch
    for ReadRecordingCodeStorage<'_, C>
{
    fn interrupt_requested(&self) -> bool {
        self.code_storage.interrupt_requested()
    }
}
