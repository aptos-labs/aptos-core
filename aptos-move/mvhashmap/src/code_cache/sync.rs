// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::TxnIndex;
use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use hashbrown::HashMap;
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::vm_status::StatusCode;
use parking_lot::lock_api;
use std::{
    hash::{Hash, RandomState},
    ops::Deref,
    sync::Arc,
};

pub struct SyncCodeCache<K, V, S> {
    script_cache: DashMap<[u8; 32], CachePadded<S>>,
    module_cache: ModuleCache<K, V>,
}

impl<K: Eq + Hash + Clone, V: Clone, S: Clone> SyncCodeCache<K, V, S> {
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
            module_cache: ModuleCache::empty(),
        }
    }

    pub fn store_script(&self, hash: [u8; 32], script: S) {
        self.script_cache.insert(hash, CachePadded::new(script));
    }

    /// Returns a script if it exists in cache, and [None] otherwise.
    pub fn fetch_script(&self, hash: &[u8; 32]) -> Option<S> {
        Some(self.script_cache.get(hash)?.clone().into_inner())
    }

    /// Returns the module cache.
    pub fn module_cache(&self) -> &ModuleCache<K, V> {
        &self.module_cache
    }
}

pub struct MaybeCommitted<V> {
    value: V,
    commit_idx: Option<TxnIndex>,
}

impl<V> MaybeCommitted<V> {
    pub fn verified(value: V, commit_idx: Option<TxnIndex>) -> Self {
        Self { value, commit_idx }
    }

    pub fn base(value: V) -> Self {
        Self {
            value,
            commit_idx: None,
        }
    }

    pub fn committed(value: V, txn_idx: TxnIndex) -> Self {
        Self {
            value,
            commit_idx: Some(txn_idx),
        }
    }

    pub fn commit_idx(&self) -> Option<TxnIndex> {
        self.commit_idx
    }
}

impl<V> Deref for MaybeCommitted<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub struct ModuleCache<K, V> {
    cache: DashMap<K, Arc<MaybeCommitted<V>>>,
}

impl<K: Eq + Hash + Clone, V: Clone> ModuleCache<K, V> {
    fn empty() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub fn num_keys(&self) -> usize {
        self.cache.len()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    pub fn check_module_commit_idx(&self, key: &K, previous_idx: Option<TxnIndex>) -> bool {
        match self.cache.get(key) {
            Some(current) => current.commit_idx() == previous_idx,
            None => false,
        }
    }

    pub fn store_base_module(&self, key: K, value: V) {
        self.cache
            .insert(key, Arc::new(MaybeCommitted::base(value)));
    }

    pub fn store_committed_module(&self, key: K, value: V, txn_idx: TxnIndex) {
        self.cache
            .insert(key, Arc::new(MaybeCommitted::committed(value, txn_idx)));
    }

    pub fn fetch_or_initialize<F>(
        &self,
        key: &K,
        init_func: &F,
    ) -> VMResult<Option<Arc<MaybeCommitted<V>>>>
    where
        F: Fn(&K) -> VMResult<Option<V>>,
    {
        if let Some(v) = self.cache.get(key) {
            return Ok(Some(v.clone()));
        }

        // This takes the write lock!
        match self.cache.entry(key.clone()) {
            Entry::Occupied(entry) => {
                // In case the entry is already initialized, return it. We must not re-initialize
                // it with the base value because there can be a transaction committing this value
                // at the same time and caching it first. Hence, that "newer" value needs to stay.
                Ok(Some(entry.get().clone()))
            },
            Entry::Vacant(entry) => match init_func(key)? {
                Some(v) => {
                    let v = Arc::new(MaybeCommitted::base(v));
                    entry.insert(v.clone());
                    Ok(Some(v))
                },
                None => Ok(None),
            },
        }
    }

    pub fn lock(&self) -> LockedModuleCache<K, V> {
        let mut locked_cache_shards = vec![];
        for shard in self.cache.shards() {
            let lock = shard.write();
            locked_cache_shards.push(lock);
        }
        LockedModuleCache {
            cache: &self.cache,
            locked_cache_shards,
        }
    }

    pub fn filter_into<T, P, F>(&self, collector: &mut HashMap<K, T>, p: P, f: F)
    where
        P: Fn(&V) -> bool,
        F: Fn(&V) -> T,
    {
        for r in self.cache.iter().filter(|r| p(r.value())) {
            collector.insert(r.key().clone(), f(r.value()));
        }
    }
}

pub type HashMapShard<K, V> = HashMap<K, dashmap::SharedValue<Arc<MaybeCommitted<V>>>, RandomState>;
pub type HashMapShardWriteGuard<'a, K, V> =
    lock_api::RwLockWriteGuard<'a, dashmap::RawRwLock, HashMapShard<K, V>>;

pub struct LockedModuleCache<'a, K, V> {
    cache: &'a DashMap<K, Arc<MaybeCommitted<V>>>,
    locked_cache_shards: Vec<HashMapShardWriteGuard<'a, K, V>>,
}

impl<'a, K: Eq + Hash + Clone, V: Clone> LockedModuleCache<'a, K, V> {
    pub fn unlock(self) {
        for lock in self.locked_cache_shards {
            drop(lock)
        }
    }

    pub fn store(&mut self, key: &K, value: Arc<MaybeCommitted<V>>) -> VMResult<()> {
        let shard_idx = self.cache.determine_shard(self.cache.hash_usize(key));
        let shard = self.locked_cache_shards.get_mut(shard_idx).ok_or_else(|| {
            PartialVMError::new(StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR)
                .with_message("Module cache access for DashMap shard is out of bounds".to_string())
                .finish(Location::Undefined)
        })?;

        shard.insert(key.clone(), dashmap::SharedValue::new(value));
        Ok(())
    }

    pub fn fetch_or_initialize<F>(
        &mut self,
        key: &K,
        init_func: &F,
    ) -> VMResult<Option<Arc<MaybeCommitted<V>>>>
    where
        F: Fn(&K) -> VMResult<Option<V>>,
    {
        let shard_idx = self.cache.determine_shard(self.cache.hash_usize(key));
        let shard = self.locked_cache_shards.get_mut(shard_idx).ok_or_else(|| {
            PartialVMError::new(StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR)
                .with_message("Module cache access for DashMap shard is out of bounds".to_string())
                .finish(Location::Undefined)
        })?;
        match shard.get(key) {
            Some(s) => Ok(Some(s.get().clone())),
            None => match init_func(key)? {
                Some(v) => {
                    let v = Arc::new(MaybeCommitted::base(v));
                    shard.insert(key.clone(), dashmap::SharedValue::new(v.clone()));
                    Ok(Some(v))
                },
                None => Ok(None),
            },
        }
    }
}
