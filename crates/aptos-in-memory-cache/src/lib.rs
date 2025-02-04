// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::hash::Hash;

pub mod caches;

/// A struct that holds a single cache entry, containing its key, value, and size in bytes.
#[derive(Clone, Debug)]
pub struct SizedCacheEntry<V> {
    pub key: usize,
    pub value: V,
    pub size_in_bytes: usize,
}

/// A trait for a cache that can be used to store key-value pairs.
pub trait SizedCache<V>: Send + Sync
where
    V: Send + Sync,
{
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    fn get(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Inserts a given key-value pair in cache. Returns the index of the inserted entry. Panics if the insert fails.
    fn insert_with_size(&self, key: usize, value: V, size_in_bytes: usize) -> usize;

    /// Evicts the value for a given key from the cache.
    fn evict(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;

    /// Returns the capacity of the cache.
    fn capacity(&self) -> usize;
}

/// A helper trait for a cache that can be used to store key-value pairs.
pub trait Cache<K, V>: Send + Sync
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    fn get(&self, key: &K) -> Option<V>;

    /// Inserts a given key-value pair in cache. Panics if the insert fails.
    fn insert(&self, key: K, value: V);

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;
}
