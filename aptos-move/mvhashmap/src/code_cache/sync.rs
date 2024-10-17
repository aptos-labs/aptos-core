// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::VersionedModule;
use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use hashbrown::HashMap;
use move_vm_types::code::{ModuleCache, ScriptCache};
use std::{hash::Hash, ops::Deref, sync::Arc};

/// A per-block code cache to be used for parallel transaction execution.
pub struct SyncCodeCache<K, M, Q, S> {
    /// Script cache, indexed by keys (e.g., hashes).
    script_cache: DashMap<Q, CachePadded<S>>,
    module_cache: SyncModuleCache<K, M>,
}

impl<K, M, Q, S> SyncCodeCache<K, M, Q, S>
where
    K: Eq + Hash + Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    /// Returns new empty code cache.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
            module_cache: SyncModuleCache::empty(),
        }
    }

    /// Returns the number of scripts stored in cache.
    pub fn num_scripts(&self) -> usize {
        self.script_cache.len()
    }

    /// Returns the module cache.
    pub fn module_cache(&self) -> &SyncModuleCache<K, M> {
        &self.module_cache
    }
}

impl<K, M, Q, S> ScriptCache for SyncCodeCache<K, M, Q, S>
where
    K: Eq + Hash + Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    type Key = Q;
    type Script = S;

    fn insert_script(&self, key: Self::Key, script: Self::Script) {
        self.script_cache.insert(key, CachePadded::new(script));
    }

    fn get_script(&self, key: &Self::Key) -> Option<Self::Script> {
        Some(self.script_cache.get(key)?.clone().into_inner())
    }
}

/// A per-block module cache to be used for parallel transaction execution.
pub struct SyncModuleCache<K, M> {
    cache: DashMap<K, CachePadded<Arc<VersionedModule<M>>>>,
}

impl<K, M> SyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    /// Returns a new empty module cache.
    fn empty() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Returns the number of modules stored in cache.
    pub fn num_modules(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the module cache contains an entry.
    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    /// Returns true if the module cache contains an entry that satisfies the predicate.
    pub fn contains_and<P>(&self, key: &K, p: P) -> bool
    where
        P: FnOnce(&VersionedModule<M>) -> bool,
    {
        self.cache
            .get(key)
            .is_some_and(|current| p(current.value()))
    }

    pub fn filter_into<T, P, F>(&self, collector: &mut HashMap<K, T>, p: P, f: F)
    where
        P: Fn(&M) -> bool,
        F: Fn(&M) -> T,
    {
        for r in self.cache.iter().filter(|r| p(r.value())) {
            collector.insert(r.key().clone(), f(r.value()));
        }
    }
}

impl<K, M> ModuleCache for SyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    type Key = K;
    type Module = VersionedModule<M>;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        self.cache.insert(key, CachePadded::new(Arc::new(module)));
    }

    fn get_module_or_insert_with<F, E>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Arc<Self::Module>>, E>
    where
        F: FnOnce() -> Result<Option<Self::Module>, E>,
    {
        if let Some(m) = self.cache.get(key) {
            return Ok(Some(m.value().deref().clone()));
        }

        // This takes the write lock!
        match self.cache.entry(key.clone()) {
            Entry::Occupied(entry) => {
                // In case the entry is already initialized, return it. We must not re-initialize
                // it with the base value because there can be a transaction committing this value
                // at the same time and caching it first. Hence, that "newer" value needs to stay.
                Ok(Some(entry.get().deref().clone()))
            },
            Entry::Vacant(entry) => match default()? {
                Some(m) => {
                    let m = Arc::new(m);
                    entry.insert(CachePadded::new(m.clone()));
                    Ok(Some(m))
                },
                None => Ok(None),
            },
        }
    }
}
