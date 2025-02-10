// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::hash::Hash;

pub mod caches;

/// A struct that holds a single cache entry, containing its key, value, and size in bytes.
#[derive(Clone, Debug)]
pub struct SizedCacheEntry<K, V> {
    pub key: K,
    pub value: V,
    pub size_in_bytes: usize,
}

/// A trait for a fixed capacity cache where size is tracked and values are keyed by [`usize`].
pub trait SizedCache<K, V>: Send + Sync {
    /// Get the value for a given [`usize`] key.
    /// Return [`None`] if the key is not in the cache.
    fn get(&self, key: &K) -> Option<SizedCacheEntry<K, V>>;

    /// Inserts a given [`usize`] key-value pair and its size into the cache.
    /// Returns the index of the inserted entry.
    fn insert_with_size(&self, key: K, value: V, size_in_bytes: usize) -> usize;

    /// Evicts the value for a given [`usize`] key from the cache.
    fn evict(&self, key: &K) -> Option<SizedCacheEntry<K, V>>;

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;

    /// Returns the maximum number of items the cache can hold at once.
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
