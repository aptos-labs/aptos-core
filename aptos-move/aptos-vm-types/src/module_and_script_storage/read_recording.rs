// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::duplicated_attributes)]

use crate::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::{ambassador_impl_BlockSynchronizationKillSwitch, BlockSynchronizationKillSwitch},
};
use ambassador::delegate_to_methods;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use bytes::Bytes;
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
};
use move_vm_runtime::{
    ambassador_impl_LayoutCache, ambassador_impl_WithRuntimeEnvironment, LayoutCache,
    LayoutCacheEntry, Module, ModuleStorage, RuntimeEnvironment, Script, StructKey,
    WithRuntimeEnvironment,
};
use move_vm_types::code::{ambassador_impl_ScriptCache, Code, ScriptCache};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{cell::RefCell, sync::Arc};

/// Wraps a code storage and records every module the VM fetches through it, so that module
/// accesses become part of the transaction's observed read set (the basis for hot state
/// promotion).
///
/// The VM fetches the same module repeatedly within a transaction, and interning a module
/// `StateKey` goes through a global, lock-guarded registry. To keep that off the per-access
/// hot path, reads are accumulated as deduplicated module ids and interned into `StateKey`s
/// once, when the set is extracted.
///
/// Scripts are not state items, so script cache accesses are not recorded.
pub struct ReadRecordingCodeStorage<'a, C> {
    code_storage: &'a C,
    module_reads: RefCell<FxHashMap<AccountAddress, FxHashSet<Identifier>>>,
}

impl<'a, C> ReadRecordingCodeStorage<'a, C> {
    pub fn new(code_storage: &'a C) -> Self {
        Self {
            code_storage,
            module_reads: RefCell::new(FxHashMap::default()),
        }
    }

    /// Returns the state keys of modules fetched so far. Already deduplicated by
    /// (address, name), so the caller receives each key once.
    pub fn into_recorded_reads(self) -> impl Iterator<Item = StateKey> {
        self.module_reads
            .into_inner()
            .into_iter()
            .flat_map(|(address, names)| {
                names
                    .into_iter()
                    .map(move |name| StateKey::module(&address, &name))
            })
    }

    fn record(&self, address: &AccountAddress, module_name: &IdentStr) {
        let mut module_reads = self.module_reads.borrow_mut();
        let names = module_reads.entry(*address).or_default();
        // `contains` borrows the name, so repeated fetches of an already-seen module
        // allocate nothing; clone only on first sighting.
        if !names.contains(module_name) {
            names.insert(module_name.to_owned());
        }
    }
}

#[delegate_to_methods]
#[delegate(
    WithRuntimeEnvironment,
    target_ref = "inner",
    where = "C: WithRuntimeEnvironment"
)]
#[delegate(LayoutCache, target_ref = "inner", where = "C: LayoutCache")]
#[delegate(
    BlockSynchronizationKillSwitch,
    target_ref = "inner",
    where = "C: BlockSynchronizationKillSwitch"
)]
impl<C> ReadRecordingCodeStorage<'_, C> {
    /// Returns the wrapped code storage.
    fn inner(&self) -> &C {
        self.code_storage
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

#[delegate_to_methods]
#[delegate(ScriptCache, target_ref = "as_script_cache")]
impl<C> ReadRecordingCodeStorage<'_, C>
where
    C: ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>,
{
    /// Returns the wrapped script cache.
    fn as_script_cache(
        &self,
    ) -> &dyn ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script> {
        self.code_storage
    }
}
