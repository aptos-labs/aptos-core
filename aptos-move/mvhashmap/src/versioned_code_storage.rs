// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scripts::ScriptCacheEntry, types::TxnIndex, versioned_module_storage::VersionedModuleStorage,
};
use aptos_types::{
    delayed_fields::PanicError, executable::ModulePath, vm::modules::ModuleStorageEntry,
    write_set::TransactionWrite,
};
use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use move_binary_format::file_format::CompiledScript;
use move_vm_runtime::{RuntimeEnvironment, Script};
use std::{fmt::Debug, hash::Hash, sync::Arc};

/// Code storage, that holds script cache and (versioned) module storage.
pub struct VersionedCodeStorage<K> {
    /// Caches deserialized and verified scripts. In the current cache
    /// implementation it is flushed on any module upgrade.
    script_cache: DashMap<[u8; 32], CachePadded<ScriptCacheEntry>>,
    /// Stores modules and pending code publishes observed by the Block-STM.
    module_storage: VersionedModuleStorage<K>,
}

impl<K: Debug + Hash + Clone + Eq + ModulePath> VersionedCodeStorage<K> {
    /// Returns a new empty versioned code storage.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
            module_storage: VersionedModuleStorage::empty(),
        }
    }

    /// Tries to get a deserialized script if it exists in cache. If not, returns [None].
    pub fn get_deserialized_script(&self, hash: &[u8; 32]) -> Option<Arc<CompiledScript>> {
        let entry = self.script_cache.get(hash)?;
        let e = &**entry.value();
        Some(e.as_compiled_script())
    }

    /// Stores the deserialized script to script cache.
    pub fn cache_deserialized_script(&self, hash: [u8; 32], compiled_script: Arc<CompiledScript>) {
        use ScriptCacheEntry::*;
        self.script_cache
            .insert(hash, CachePadded::new(Deserialized(compiled_script)));
    }

    /// Tries to get a verified script if it exists in cache. If not, returns [None].
    /// If the deserialized version exists instead.
    pub fn get_verified_script(
        &self,
        hash: &[u8; 32],
    ) -> Option<Result<Arc<Script>, Arc<CompiledScript>>> {
        let entry = self.script_cache.get(hash)?;

        use ScriptCacheEntry::*;
        Some(match &**entry.value() {
            Verified(script) => Ok(script.clone()),
            Deserialized(compiled_script) => Err(compiled_script.clone()),
        })
    }

    /// Stores the verified script to script cache, unless it already exists.
    pub fn cache_verified_script(&self, hash: [u8; 32], script: Arc<Script>) {
        let entry = self.script_cache.entry(hash);

        use ScriptCacheEntry::*;
        if let Entry::Occupied(mut e) = entry {
            if !e.get().is_verified() {
                e.insert(CachePadded::new(Verified(script)));
            }
        }
    }

    /// Writes multiple published modules to the storage, which is also visible for
    /// the transactions with higher indices. This function invalidates and flushes
    /// the script cache.
    pub fn write_published_modules<V: TransactionWrite>(
        &self,
        idx_to_publish: TxnIndex,
        runtime_environment: &RuntimeEnvironment,
        writes: impl Iterator<Item = (K, V)>,
    ) -> Result<(), PanicError> {
        // In case of module publishing, flush script cache. This is the simplest thing
        // to do for now and can be improved if needed.
        self.script_cache.clear();

        for (key, write) in writes {
            let entry = ModuleStorageEntry::from_transaction_write(runtime_environment, write)?;
            self.module_storage
                .write_published(&key, idx_to_publish, entry);
        }
        Ok(())
    }

    pub fn module_storage(&self) -> &VersionedModuleStorage<K> {
        &self.module_storage
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2): Add tests here.
}
