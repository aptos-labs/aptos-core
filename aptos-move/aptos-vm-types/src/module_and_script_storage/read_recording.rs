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

thread_local! {
    /// A module's `StateKey` is a pure function of its `(address, name)` and never changes, yet
    /// interning one goes through the global, lock-guarded `StateKey` registry. Worker threads
    /// re-read the same modules on every transaction, so memoize the interned keys per thread to
    /// keep that lock off the extraction path. Bounded so an unbounded module working set can't
    /// grow it without limit; the hot (framework) modules are seen first and stay cached.
    static MODULE_STATE_KEYS: RefCell<FxHashMap<AccountAddress, FxHashMap<Identifier, StateKey>>> =
        RefCell::new(FxHashMap::default());
}

/// Cap on distinct addresses memoized per thread, bounding the memo under a very large or
/// adversarial module working set.
const MODULE_STATE_KEY_CACHE_MAX_ADDRESSES: usize = 1 << 13;

/// Interns a module `StateKey`, serving repeats from the per-thread memo so that the common case
/// (a module already seen by this thread) avoids the global registry lock entirely.
fn interned_module_state_key(address: &AccountAddress, name: &IdentStr) -> StateKey {
    MODULE_STATE_KEYS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(key) = cache.get(address).and_then(|names| names.get(name)) {
            return key.clone();
        }
        let key = StateKey::module(address, name);
        // Only new addresses are gated by the cap; an already-cached address keeps accumulating
        // its modules so it is never left half-populated.
        if cache.len() < MODULE_STATE_KEY_CACHE_MAX_ADDRESSES || cache.contains_key(address) {
            cache
                .entry(*address)
                .or_default()
                .insert(name.to_owned(), key.clone());
        }
        key
    })
}

/// Wraps a code storage and records every module the VM fetches through it, so that module
/// accesses become part of the transaction's observed read set (the basis for hot state
/// promotion).
///
/// Recorded reads are kept directly as interned `StateKey`s served from the per-thread memo,
/// so steady state a transaction's recording performs no allocation beyond growing its key
/// set: no owned module ids, and no interning pass at extraction.
///
/// Scripts are not state items, so script cache accesses are not recorded.
pub struct ReadRecordingCodeStorage<'a, C> {
    code_storage: &'a C,
    module_reads: RefCell<FxHashSet<StateKey>>,
    /// The previously recorded `(address, name)`. The interpreter fetches the same module many
    /// times in a row, so this lets a burst of accesses skip the memo and set lookups. A
    /// `String` buffer rather than an `Identifier` so updating it on a module switch reuses
    /// the allocation instead of making a fresh one.
    last_recorded: RefCell<(AccountAddress, String)>,
}

impl<'a, C> ReadRecordingCodeStorage<'a, C> {
    pub fn new(code_storage: &'a C) -> Self {
        Self {
            code_storage,
            // Even a trivial transaction touches 10+ framework modules through its prologue
            // and epilogue, so start with room for the typical count and skip the rehashes.
            module_reads: RefCell::new(FxHashSet::with_capacity_and_hasher(24, Default::default())),
            // Module names are never empty, so an empty name means "nothing recorded yet".
            last_recorded: RefCell::new((AccountAddress::ZERO, String::new())),
        }
    }

    /// Returns the state keys of modules fetched so far, deduplicated by key.
    pub fn into_recorded_reads(self) -> FxHashSet<StateKey> {
        self.module_reads.into_inner()
    }

    #[inline]
    fn record(&self, address: &AccountAddress, module_name: &IdentStr) {
        {
            // Fast path: a run of accesses to the same module needs no further work. Only an
            // exact (address, name) match is skipped, so the recorded set is identical either
            // way.
            let last = self.last_recorded.borrow();
            if last.0 == *address && last.1.as_str() == module_name.as_str() {
                return;
            }
        }
        let key = interned_module_state_key(address, module_name);
        self.module_reads.borrow_mut().insert(key);
        let mut last = self.last_recorded.borrow_mut();
        last.0 = *address;
        last.1.clear();
        last.1.push_str(module_name.as_str());
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
    #[inline]
    fn unmetered_check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_check_module_exists(address, module_name)
    }

    #[inline]
    fn unmetered_get_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_bytes(address, module_name)
    }

    #[inline]
    fn unmetered_get_module_hash_and_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<([u8; 32], usize)>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_hash_and_size(address, module_name)
    }

    #[inline]
    fn unmetered_get_module_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_module_size(address, module_name)
    }

    #[inline]
    fn unmetered_get_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_deserialized_module(address, module_name)
    }

    #[inline]
    fn unmetered_get_eagerly_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        self.record(address, module_name);
        self.code_storage
            .unmetered_get_eagerly_verified_module(address, module_name)
    }

    #[inline]
    fn unmetered_get_lazily_verified_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<Module>>> {
        self.record(module_id.address(), module_id.name());
        self.code_storage
            .unmetered_get_lazily_verified_module(module_id)
    }

    #[cfg(fuzzing)]
    #[inline]
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
    #[inline]
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
