// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{Cache, Incrementable, Ordered};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::hash::Hash;

#[derive(Debug, Clone, Copy)]
struct CacheMetadata<K> {
    max_size_in_bytes: u64,
    total_size_in_bytes: u64,
    last_key: Option<K>,
    first_key: Option<K>,
}

/// FIFO is a simple in-memory cache with a deterministic FIFO eviction policy.
pub struct FIFOCache<K, V>
where
    K: Hash + Eq + PartialEq + Incrementable<V> + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    /// Cache maps the cache key to the deserialized Transaction.
    items: DashMap<K, V>,
    cache_metadata: RwLock<CacheMetadata<K>>,
}

impl<K, V> FIFOCache<K, V>
where
    K: Hash + Eq + PartialEq + Incrementable<V> + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    pub fn new(max_size_in_bytes: u64) -> Self {
        FIFOCache {
            items: DashMap::new(),
            cache_metadata: RwLock::new(CacheMetadata {
                max_size_in_bytes,
                total_size_in_bytes: 0,
                last_key: None,
                first_key: None,
            }),
        }
    }

    fn pop(&self) -> u64 {
        let mut cache_metadata = self.cache_metadata.write();
        let first_key = cache_metadata.first_key.clone().unwrap();
        let (k, v) = self.items.remove(&first_key).unwrap(); // cleanup
        cache_metadata.first_key = Some(k.next(&v));
        let weight = std::mem::size_of_val(&v) as u64;
        cache_metadata.total_size_in_bytes -= weight;
        weight
    }

    fn evict(&self, new_value_weight: u64) -> (u64, u64) {
        let mut garbage_collection_count = 0;
        let mut garbage_collection_size = 0;
        loop {
            let cache_metadata = self.cache_metadata.read();
            if cache_metadata.total_size_in_bytes + new_value_weight
                <= cache_metadata.max_size_in_bytes
            {
                break;
            }
            drop(cache_metadata);
            let weight = self.pop();
            garbage_collection_count += 1;
            garbage_collection_size += weight;
        }
        (garbage_collection_count, garbage_collection_size)
    }

    fn insert_impl(&self, key: K, value: V) {
        let mut cache_metadata = self.cache_metadata.write();
        cache_metadata.last_key = Some(key.clone());
        cache_metadata.total_size_in_bytes += std::mem::size_of_val(&value) as u64;
        self.items.insert(key, value);
    }
}

impl<K, V> Cache<K, V> for FIFOCache<K, V>
where
    K: Hash + Eq + PartialEq + Incrementable<V> + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        self.items.get(key).map(|v| v.value().clone())
    }

    fn insert(&self, key: K, value: V) -> (u64, u64) {
        // If cache is empty, set the first to the new key.
        if self.items.is_empty() {
            let mut cache_metadata = self.cache_metadata.write();
            cache_metadata.first_key = Some(key.clone());
        }

        // Evict until enough space is available for next value.
        let (garbage_collection_count, garbage_collection_size) =
            self.evict(std::mem::size_of_val(&value) as u64);
        self.insert_impl(key, value);

        return (garbage_collection_count, garbage_collection_size);
    }

    fn total_size(&self) -> u64 {
        let cache_metadata = self.cache_metadata.read();
        cache_metadata.total_size_in_bytes
    }
}

impl<K, V> Ordered<K> for FIFOCache<K, V>
where
    K: Hash + Eq + PartialEq + Incrementable<V> + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    fn first_key(&self) -> Option<K> {
        let cache_metadata = self.cache_metadata.read();
        cache_metadata.first_key.clone()
    }

    fn last_key(&self) -> Option<K> {
        let cache_metadata = self.cache_metadata.read();
        cache_metadata.last_key.clone()
    }
}
