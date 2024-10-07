// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::versioned_module_storage::VersionedModuleStorage;
use aptos_types::vm::{modules::ModuleStorageEntry, scripts::ScriptCacheEntry};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use move_core_types::language_storage::ModuleId;

/// Code storage, that holds script cache and (versioned) module storage.
pub struct VersionedCodeStorage {
    /// Caches deserialized and verified scripts. In the current cache
    /// implementation it is flushed on any module upgrade.
    // TODO(loader-V2): do we need to flush?
    script_cache: DashMap<[u8; 32], CachePadded<ScriptCacheEntry>>,
    /// Stores modules and pending code publishes observed by the Block-STM.
    module_storage: VersionedModuleStorage<ModuleId, ModuleStorageEntry>,
}

impl VersionedCodeStorage {
    /// Returns a new empty versioned code storage.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
            module_storage: VersionedModuleStorage::empty(),
        }
    }

    /// Stores the deserialized script to script cache.
    pub fn cache_script(&self, hash: [u8; 32], script: ScriptCacheEntry) {
        self.script_cache.insert(hash, CachePadded::new(script));
    }

    /// Tries to get a verified script if it exists in cache. If not, returns [None].
    /// If the deserialized version exists instead.
    pub fn fetch_cached_script(&self, hash: &[u8; 32]) -> Option<ScriptCacheEntry> {
        Some(self.script_cache.get(hash)?.clone().into_inner())
    }

    pub fn module_storage(&self) -> &VersionedModuleStorage<ModuleId, ModuleStorageEntry> {
        &self.module_storage
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2): Add tests here.
}
