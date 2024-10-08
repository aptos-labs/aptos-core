// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm::{modules::ModuleCacheEntry, scripts::ScriptCacheEntry};
use hashbrown::HashMap;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use std::{cell::RefCell, sync::Arc};

/// A per-block code cache to be used for sequential transaction execution. Modules and scripts
/// can be cached and retrieved. It is responsibility of the caller to cache the base (i.e.,
/// storage version) modules.
pub struct UnsyncCodeCache {
    /// Script cache, indexed by script hashes.
    script_cache: RefCell<HashMap<[u8; 32], ScriptCacheEntry>>,
    /// Module cache, indexed by module address-name pair.
    module_cache: RefCell<HashMap<ModuleId, Arc<ModuleCacheEntry>>>,
}

impl UnsyncCodeCache {
    /// Returns an empty code cache.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: RefCell::new(HashMap::new()),
            module_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns the number of modules cached in the code cache.
    pub(crate) fn num_modules(&self) -> usize {
        self.module_cache.borrow().len()
    }

    /// Stores the module to the code cache.
    pub fn cache_module(&self, module_id: ModuleId, entry: Arc<ModuleCacheEntry>) {
        self.module_cache.borrow_mut().insert(module_id, entry);
    }

    /// Fetches the module from the code cache, if it exists there. Otherwise, returns [None].
    pub fn fetch_cached_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<Arc<ModuleCacheEntry>> {
        self.module_cache
            .borrow()
            .get(&(address, module_name))
            .cloned()
    }

    /// Stores the script to the code cache.
    pub fn cache_script(&self, hash: [u8; 32], entry: ScriptCacheEntry) {
        self.script_cache.borrow_mut().insert(hash, entry);
    }

    /// Returns the script if it has been cached before, or [None] otherwise.
    pub fn fetch_cached_script(&self, hash: &[u8; 32]) -> Option<ScriptCacheEntry> {
        self.script_cache.borrow().get(hash).cloned()
    }
}
