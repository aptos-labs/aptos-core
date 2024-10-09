// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::state_value::StateValueMetadata,
    vm::{modules::ModuleCacheEntry, scripts::ScriptCacheEntry},
};
use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use hashbrown::HashMap;
use move_binary_format::errors::VMResult;
use move_core_types::language_storage::ModuleId;
use std::sync::Arc;

/// Code cache that stores scripts and modules, and can be used by Block-STM parallel execution.
pub struct SyncCodeCache {
    script_cache: DashMap<[u8; 32], CachePadded<ScriptCacheEntry>>,
    module_cache: ModuleCache,
}

impl SyncCodeCache {
    /// Returns a new empty code cache.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
            module_cache: ModuleCache::empty(),
        }
    }

    /// Stores the script to the code cache.
    pub fn cache_script(&self, hash: [u8; 32], script: ScriptCacheEntry) {
        self.script_cache.insert(hash, CachePadded::new(script));
    }

    /// Returns a script if it exists in cache, and [None] otherwise.
    pub fn fetch_cached_script(&self, hash: &[u8; 32]) -> Option<ScriptCacheEntry> {
        Some(self.script_cache.get(hash)?.clone().into_inner())
    }

    /// Returns the module cache.
    pub fn module_cache(&self) -> &ModuleCache {
        &self.module_cache
    }
}

/// Per-block mutable module cache, that can be used by transactions executed in parallel. It is
/// responsibility of the Block-STM to ensure that the cache is kept consistent. In particular, it
/// is important to make sure that entries are added only if:
///   1. Transaction published modules and is being committed. The module entry can be inserted in
///      this case because it is not speculative, and it is ok for higher-indexed transactions to
///      see the new code. The new code cannot be overwritten because only one  transaction is
///      committed at a time.
///   2. Transaction loads a module from storage. Then, it is not possible for lower-indexed
///      transaction to override published module because it must have been committed and finished
///      the execution before the other transaction can be scheduled for the commit.
pub struct ModuleCache {
    cache: DashMap<ModuleId, Arc<ModuleCacheEntry>>,
}

impl ModuleCache {
    /// Returns a new empty module cache.
    fn empty() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Returns the number of modules currently stored in cache.
    pub fn num_keys(&self) -> usize {
        self.cache.len()
    }

    /// Stores the module to the code cache.
    pub fn cache_module(&self, module_id: ModuleId, entry: Arc<ModuleCacheEntry>) {
        self.cache.insert(module_id, entry);
    }

    /// Returns true if the module cache currently contains the module with the same hash and the
    /// same state value metadata as previously, or does not contain the cached entry in case it
    /// did not contain it before. Used to validate module storage reads when there are modules
    /// published.
    /// Note that [StateValueMetadata] comparison is required because it is currently possible to
    /// publish modules with the same code (e.g., changing a single module as part of a package,
    /// but publishing the whole package), and hence the same cache. For those, the state value
    /// metadata changes, at least because we no longer have a creation but a modification op.
    pub fn check_hash_and_state_value_metadata(
        &self,
        module_id: &ModuleId,
        previous_hash_and_state_value_metadata: Option<(&[u8; 32], &StateValueMetadata)>,
    ) -> bool {
        let current_hash_and_state_value_metadata = self
            .cache
            .get(module_id)
            .map(|e| (*e.hash(), e.state_value_metadata().clone()));
        match (
            &current_hash_and_state_value_metadata,
            previous_hash_and_state_value_metadata,
        ) {
            (
                Some((current_hash, current_state_value_metadata)),
                Some((previous_hash, previous_state_value_metadata)),
            ) => {
                current_hash == previous_hash
                    && current_state_value_metadata == previous_state_value_metadata
            },
            (None, None) => true,
            (Some(..), None) | (None, Some(..)) => false,
        }
    }

    /// Return the cached module from the module cache. If it is not cached, use the passed
    /// initialization function to initialize the entry. The initialization is done under the lock.
    pub fn fetch_cached_module_or_initialize<F>(
        &self,
        module_id: &ModuleId,
        init_func: F,
    ) -> VMResult<Option<Arc<ModuleCacheEntry>>>
    where
        F: Fn() -> VMResult<Option<Arc<ModuleCacheEntry>>>,
    {
        if let Some(entry) = self.cache.get(module_id) {
            return Ok(Some(entry.clone()));
        }

        // This takes the write lock!
        match self.cache.entry(module_id.clone()) {
            Entry::Occupied(e) => {
                // In case the entry is already initialized, return it. We must not re-initialize
                // it with the storage version because there can be a transaction committing this
                // module (republish) at the same time and caching it first. Hence, that "newer"
                // code needs to stay in cache.
                Ok(Some(e.get().clone()))
            },
            Entry::Vacant(e) => {
                let maybe_entry = init_func()?;
                if let Some(entry) = &maybe_entry {
                    e.insert(entry.clone());
                }
                Ok(maybe_entry)
            },
        }
    }

    /// Collects the verified modules that were published and loaded during this block. Should only
    /// be called at the block end.
    pub fn collect_verified_entries_into<F, V>(&self, collector: &mut HashMap<ModuleId, V>, f: F)
    where
        F: Fn(&ModuleCacheEntry) -> V,
    {
        for r in self.cache.iter().filter(|r| r.value().is_verified()) {
            collector.insert(r.key().clone(), f(r.value().as_ref()));
        }
    }
}
