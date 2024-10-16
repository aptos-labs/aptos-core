// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use hashbrown::HashMap;
use move_vm_types::code::{ModuleCache, ScriptCache};
use parking_lot::lock_api;
use std::{
    cell::RefCell,
    hash::{Hash, RandomState},
    marker::PhantomData,
    ops::Deref,
};

/// A per-block code cache to be used for parallel transaction execution.
pub struct SyncCodeCache<K, M, Q, S, E> {
    /// Script cache, indexed by keys (e.g., hashes).
    script_cache: DashMap<Q, CachePadded<S>>,
    /// Thread-safe module cache.
    module_cache: SyncModuleCache<K, M, E>,
}

impl<K, M, Q, S, E> SyncCodeCache<K, M, Q, S, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
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
    pub fn module_cache(&self) -> &SyncModuleCache<K, M, E> {
        &self.module_cache
    }
}

impl<K, M, Q, S, E> ScriptCache for SyncCodeCache<K, M, Q, S, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
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
pub struct SyncModuleCache<K, M, E> {
    cache: DashMap<K, CachePadded<M>>,
    phantom_data: PhantomData<E>,
}

impl<K, M, E> SyncModuleCache<K, M, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
{
    /// Returns a new empty module cache.
    fn empty() -> Self {
        Self {
            cache: DashMap::new(),
            phantom_data: PhantomData,
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
        P: FnOnce(&M) -> bool,
    {
        match self.cache.get(key) {
            Some(current) => p(current.value()),
            None => false,
        }
    }

    /// Locks the module cache, and returns a guard.
    pub fn lock(&self) -> LockedSyncModuleCache<K, M, E> {
        let mut locked_cache_shards = vec![];
        for shard in self.cache.shards() {
            let lock = shard.write();
            locked_cache_shards.push(lock);
        }

        // At this point all shards are locked. Only one thread can manipulate the locked cache.
        LockedSyncModuleCache {
            cache: &self.cache,
            phantom_data: self.phantom_data,
            locked_cache_shards: RefCell::new(locked_cache_shards),
        }
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

impl<K, M, E> ModuleCache for SyncModuleCache<K, M, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
{
    type Error = E;
    type Key = K;
    type Module = M;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        self.cache.insert(key, CachePadded::new(module));
    }

    fn get_module_or_insert_with<F>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Self::Module>, Self::Error>
    where
        F: FnOnce() -> Result<Option<Self::Module>, Self::Error>,
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
                    entry.insert(CachePadded::new(m.clone()));
                    Ok(Some(m))
                },
                None => Ok(None),
            },
        }
    }
}

pub type HashMapShard<K, M> = HashMap<K, dashmap::SharedValue<CachePadded<M>>, RandomState>;
pub type HashMapShardWriteGuard<'a, K, M> =
    lock_api::RwLockWriteGuard<'a, dashmap::RawRwLock, HashMapShard<K, M>>;

pub struct LockedSyncModuleCache<'a, K, M, E> {
    // Note: the reference to the dashmap is used ONLY to calculate the shard index!
    cache: &'a DashMap<K, CachePadded<M>>,
    phantom_data: PhantomData<E>,
    locked_cache_shards: RefCell<Vec<HashMapShardWriteGuard<'a, K, M>>>,
}

impl<'a, K, M, E> LockedSyncModuleCache<'a, K, M, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
{
    /// Unlocks the module cache.
    pub fn unlock(self) {
        for lock in self.locked_cache_shards.into_inner() {
            drop(lock)
        }
    }
}

impl<K, M, E> ModuleCache for LockedSyncModuleCache<'_, K, M, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
{
    type Error = E;
    type Key = K;
    type Module = M;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        let shard_idx = self.cache.determine_shard(self.cache.hash_usize(&key));
        let value = dashmap::SharedValue::new(CachePadded::new(module));
        self.locked_cache_shards
            .borrow_mut()
            .get_mut(shard_idx)
            .expect("Shard index when storing module should always be within bounds")
            .insert(key, value);
    }

    fn get_module_or_insert_with<F>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Self::Module>, Self::Error>
    where
        F: FnOnce() -> Result<Option<Self::Module>, Self::Error>,
    {
        let shard_idx = self.cache.determine_shard(self.cache.hash_usize(key));
        let mut locked_cache_shards = self.locked_cache_shards.borrow_mut();
        let shard = locked_cache_shards
            .get_mut(shard_idx)
            .expect("Shard index when fetching modules should always be within bounds");

        match shard.get(key) {
            Some(s) => Ok(Some(s.get().deref().clone())),
            None => match default()? {
                Some(m) => {
                    let value = dashmap::SharedValue::new(CachePadded::new(m.clone()));
                    shard.insert(key.clone(), value);
                    Ok(Some(m))
                },
                None => Ok(None),
            },
        }
    }
}
