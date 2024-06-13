// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{Cache, OrderedCache, StreamableOrderedCache};
use dashmap::DashMap;
use futures::{stream, Stream};
use parking_lot::RwLock;
use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Debug, Clone, Copy)]
struct CacheMetadata<K> {
    eviction_trigger_size_in_bytes: u64,
    target_size_in_bytes: u64,
    total_size_in_bytes: u64,
    last_key: Option<K>,
    first_key: Option<K>,
}

/// A simple in-memory cache with a deterministic FIFO eviction policy.
pub struct FIFOCache<K, V>
where
    K: Hash + Eq + PartialEq + Send + Sync + Clone + 'static,
    V: Send + Sync + Clone + 'static,
{
    /// Precache to support out of order inserts
    precache: Arc<DashMap<K, V>>,
    precache_insert_notify: Arc<Notify>,
    /// Cache maps the cache key to the deserialized Transaction.
    items: Arc<DashMap<K, V>>,
    insert_notify: Arc<Notify>,
    cache_metadata: Arc<RwLock<CacheMetadata<K>>>,
    _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
    // User defined function to get the next key for a given key
    // The function provides the key and the getter function to the DashMap in case a lookup is necessary
    next_key_function:
        Arc<dyn Fn(&K, &dyn Fn(&K) -> Option<V>) -> Option<K> + Send + Sync + 'static>,
}

impl<K, V> FIFOCache<K, V>
where
    K: Debug + Hash + Eq + PartialEq + Send + Sync + Clone + 'static,
    V: Send + Sync + Clone + 'static,
{
    pub fn new(
        target_size_in_bytes: u64,
        eviction_trigger_size_in_bytes: u64,
        next_key_function: impl Fn(&K, &dyn Fn(&K) -> Option<V>) -> Option<K> + Send + Sync + 'static,
    ) -> Self {
        let cancellation_token: CancellationToken = CancellationToken::new();
        let precache = Arc::new(DashMap::new());
        let precache_insert_notify = Arc::new(Notify::new());
        let items = Arc::new(DashMap::new());
        let insert_notify = Arc::new(Notify::new());
        let cache_metadata = Arc::new(RwLock::new(CacheMetadata {
            eviction_trigger_size_in_bytes,
            target_size_in_bytes,
            total_size_in_bytes: 0,
            last_key: None,
            first_key: None,
        }));
        let next_key_function = Arc::new(next_key_function);

        let cache = Self {
            precache,
            precache_insert_notify,
            items,
            insert_notify,
            cache_metadata,
            _cancellation_token_drop_guard: cancellation_token.clone().drop_guard(),
            next_key_function,
        };

        cache.spawn_insertion_task(cancellation_token.clone());
        cache.spawn_eviction_task(cancellation_token);
        cache
    }

    fn insert(
        items: Arc<DashMap<K, V>>,
        cache_metadata: Arc<RwLock<CacheMetadata<K>>>,
        insert_notify: Arc<Notify>,
        key: K,
        value: V,
    ) {
        // If cache is empty, set the first to the new key.
        if items.is_empty() {
            let mut cache_metadata = cache_metadata.write();
            cache_metadata.first_key = Some(key.clone());
        }

        let mut cache_metadata = cache_metadata.write();
        cache_metadata.last_key = Some(key.clone());
        cache_metadata.total_size_in_bytes += std::mem::size_of_val(&value) as u64;
        items.insert(key, value);
        insert_notify.notify_waiters();
    }

    fn spawn_insertion_task(&self, cancellation_token: CancellationToken) {
        let precache = self.precache.clone();
        let precache_insert_notify = self.precache_insert_notify.clone();
        let items = self.items.clone();
        let insert_notify = self.insert_notify.clone();
        let cache_metadata = self.cache_metadata.clone();
        let next_key_function = self.next_key_function.clone();

        tokio::spawn(async move {
            let cache_getter = |k: &K| -> Option<V> { items.get(k).map(|r| r.value().clone()) };
            loop {
                tokio::select! {
                    _ = precache_insert_notify.notified() => {
                        loop {
                            let key_to_insert = match cache_metadata.read().last_key {
                                Some(ref last_key) => (next_key_function)(last_key, &cache_getter),
                                None => precache.iter().next().map(|r| r.key().clone()),
                            };

                            if let Some(key) = key_to_insert {
                                if let Some((key, value)) = precache.remove(&key) {
                                    Self::insert(items.clone(), cache_metadata.clone(), insert_notify.clone(), key, value);
                                    continue;
                                }
                            }
                            break;
                        }
                    },
                    _ = cancellation_token.cancelled() => {
                        info!("In-memory cache insertion task is cancelled.");
                        return;
                    }
                }
            }
        });
    }

    fn evict(
        items: Arc<DashMap<K, V>>,
        cache_metadata: Arc<RwLock<CacheMetadata<K>>>,
        next_key_function: Arc<
            dyn Fn(&K, &dyn Fn(&K) -> Option<V>) -> Option<K> + Send + Sync + 'static,
        >,
    ) {
        // Skip if eviction is not needed.
        let should_evict = {
            let current_cache_metadata = cache_metadata.read();
            current_cache_metadata
                .total_size_in_bytes
                .saturating_sub(current_cache_metadata.eviction_trigger_size_in_bytes)
                > 0
        };
        if !should_evict {
            return;
        }

        // Evict items from the cache.
        let mut current_cache_metadata = cache_metadata.write();
        let mut actual_bytes_removed = 0;
        let mut bytes_to_remove = current_cache_metadata
            .total_size_in_bytes
            .saturating_sub(current_cache_metadata.target_size_in_bytes);
        let getter = |k: &K| -> Option<V> { items.get(k).map(|r| r.value().clone()) };
        while bytes_to_remove > 0 {
            if let Some(key_to_remove) = current_cache_metadata.first_key.clone() {
                let next_key = (next_key_function)(&key_to_remove, &getter)
                    .unwrap_or_else(|| panic!("Key after {:?} should exist.", key_to_remove));
                let (_k, v) = items
                    .remove(&key_to_remove)
                    .expect("Key to remove should exist.");
                let size_of_v = std::mem::size_of_val(&v) as u64;
                bytes_to_remove = bytes_to_remove.saturating_sub(size_of_v);
                actual_bytes_removed += size_of_v;
                current_cache_metadata.first_key = Some(next_key);
            } else {
                break;
            }
        }

        current_cache_metadata.total_size_in_bytes -= actual_bytes_removed;
    }

    /// Perform cache eviction on a separate task.
    fn spawn_eviction_task(&self, cancellation_token: CancellationToken) {
        let insert_notify = self.insert_notify.clone();
        let items = self.items.clone();
        let cache_metadata = self.cache_metadata.clone();
        let next_key_function = self.next_key_function.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = insert_notify.notified() => {
                        Self::evict(items.clone(), cache_metadata.clone(), next_key_function.clone());
                    },
                    _ = cancellation_token.cancelled() => {
                        info!("In-memory cache eviction task is cancelled.");
                        return;
                    }
                }
            }
        });
    }

    fn next_key(&self, key: &K) -> Option<K> {
        (self.next_key_function)(key, &|k| self.items.get(k).map(|r| r.value().clone()))
    }
}

impl<K, V> Cache<K, V> for FIFOCache<K, V>
where
    K: Debug + Hash + Eq + PartialEq + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        self.items.get(key).map(|v| v.value().clone())
    }

    fn insert(&self, key: K, value: V) {
        self.precache.insert(key, value);
        self.precache_insert_notify.notify_waiters();
    }

    fn total_size(&self) -> u64 {
        let cache_metadata = self.cache_metadata.read();
        cache_metadata.total_size_in_bytes
    }
}

impl<K, V> OrderedCache<K, V> for FIFOCache<K, V>
where
    K: Debug + Hash + Eq + PartialEq + Send + Sync + Clone,
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

impl<K, V> StreamableOrderedCache<K, V> for FIFOCache<K, V>
where
    K: Debug + Hash + Eq + PartialEq + Send + Sync + Clone,
    V: Send + Sync + Clone,
{
    fn next_key(&self, key: &K) -> Option<K> {
        FIFOCache::next_key(self, key)
    }

    fn next_key_and_value(&self, key: &K) -> Option<(K, V)> {
        let next_key = self.next_key(key);
        next_key.and_then(|k| self.get(&k).map(|v| (k, v)))
    }

    /// Returns a stream of values in the cache starting from the given key.
    /// If the stream falls behind, the stream will return None for the next value (indicating that it should be reset).
    fn get_stream(&self, starting_key: Option<K>) -> impl Stream<Item = V> + '_ {
        // Start from the starting key if provided, otherwise start from the last key
        let initial_state = starting_key.or_else(|| self.cache_metadata.read().last_key.clone());

        Box::pin(stream::unfold(initial_state, move |state| {
            async move {
                let mut current_key = state;
                // If the current key is None, the cache is empty
                // Wait until a new value is inserted before assigning it
                if current_key.is_none() {
                    self.insert_notify.notified().await;
                    current_key = self.cache_metadata.read().last_key.as_ref().cloned();
                }

                // These values should all exist
                let current_key = current_key.expect("Current key should exist");
                let last_key = self
                    .cache_metadata
                    .read()
                    .last_key
                    .as_ref()
                    .expect("Last key should exist")
                    .clone();
                let next_last_key = self
                    .next_key(&last_key)
                    .expect("Next last key should exist because last key exists");

                // Stream is ahead of cache
                // If the last value in the cache has already been streamed, wait until a new value is inserted and return it
                if current_key == next_last_key {
                    self.insert_notify.notified().await;
                    return Some((
                        self.get(&current_key)
                            .expect("Value should exist as it was just inserted"),
                        self.next_key(&current_key),
                    ));
                }
                // Stream is in cache bounds
                // If the next value to stream is in the cache, return it
                else if let Some(v) = self.get(&current_key) {
                    return Some((v, self.next_key(&current_key)));
                }
                // Stream is behind cache
                // If the next value to stream is not in the cache, stop the stream
                else {
                    return None;
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{caches::fifo::FIFOCache, Cache, StreamableOrderedCache};
    use futures::StreamExt;
    use std::{sync::Arc, time::Duration};

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_insert_four_values() {
        let cache = FIFOCache::<u64, u64>::new(100, 200, |key, _| Some(key + 1));
        let cache = Arc::new(cache);
        tokio::time::sleep(Duration::from_nanos(1)).await;

        cache.insert(1, 1);
        tokio::time::sleep(Duration::from_nanos(1)).await;
        cache.insert(2, 2);
        tokio::time::sleep(Duration::from_nanos(1)).await;
        cache.insert(3, 3);
        tokio::time::sleep(Duration::from_nanos(1)).await;
        cache.insert(4, 4);
        tokio::time::sleep(Duration::from_nanos(1)).await;

        assert_eq!(cache.get(&1), Some(1));
        assert_eq!(cache.get(&2), Some(2));
        assert_eq!(cache.get(&3), Some(3));
        assert_eq!(cache.get(&4), Some(4));
    }

    #[tokio::test]
    async fn test_add_ten_values_with_eviction() {
        let cache = FIFOCache::<u64, u64>::new(40, 64, |key, _| Some(key + 1));
        let cache = Arc::new(cache);
        tokio::time::sleep(Duration::from_nanos(1)).await;

        // Insert 8 values, size is 8*8=64 bytes
        for i in 0..8 {
            cache.insert(i, i);
            tokio::time::sleep(Duration::from_nanos(1)).await;
            assert_eq!(cache.total_size(), (i + 1) * 8);
            tokio::time::sleep(Duration::from_nanos(1)).await;
        }

        for i in 0..8 {
            assert_eq!(cache.get(&i), Some(i));
        }

        // Insert 9th value, size is 8*9=72>64 bytes, eviction threshold reached
        // Evicts until target size size is reached
        // Sleep for 1 ns to ensure eviction task finishes
        tokio::time::sleep(Duration::from_nanos(1)).await;
        cache.insert(8, 8);
        tokio::time::sleep(Duration::from_nanos(1)).await;
        // New size is 8*5=40 bytes
        // Keys evicted: 0, 1, 2, 3
        assert_eq!(cache.total_size(), 40);
        assert_eq!(cache.get(&0), None);
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), None);

        // Insert 10th value, size is 8*6=48 bytes
        cache.insert(9, 9);
        tokio::time::sleep(Duration::from_nanos(1)).await;
        assert_eq!(cache.total_size(), 48);
        assert_eq!(cache.get(&9), Some(9));
    }

    #[tokio::test]
    async fn test_read_from_stream() {
        let cache = FIFOCache::<u64, u64>::new(40, 64, |key, _| Some(key + 1));
        let cache = Arc::new(cache);
        tokio::time::sleep(Duration::from_nanos(1)).await;

        // Insert 8 values, size is 8*8=64 bytes
        for i in 0..8 {
            cache.insert(i, i);
            tokio::time::sleep(Duration::from_nanos(1)).await;
        }

        let mut stream = cache.get_stream(Some(0));
        tokio::time::sleep(Duration::from_nanos(1)).await;
        for i in 0..8 {
            assert_eq!(stream.next().await.unwrap(), i);
            tokio::time::sleep(Duration::from_nanos(1)).await;
        }

        // Insert 9th value, size is 8*9=72>64 bytes, eviction threshold reached
        // Evicts until target size size is reached
        // Sleep for 1 ns to ensure eviction task finishes
        tokio::time::sleep(Duration::from_nanos(1)).await;
        cache.insert(8, 8);
        tokio::time::sleep(Duration::from_nanos(1)).await;

        // New size is 8*5=40 bytes
        // Keys evicted: 0, 1, 2, 3
        let mut stream2 = cache.get_stream(Some(0));
        // The stream has fallen behind since 0 has been evicted already
        assert_eq!(stream2.next().await, None);
    }

    #[tokio::test]
    async fn test_stream_picks_up_new_inserts() {
        let cache = FIFOCache::<u64, u64>::new(40, 64, |key, _| Some(key + 1));
        let cache = Arc::new(cache);

        let cache_clone = cache.clone();
        tokio::spawn(async move {
            let mut stream = cache_clone.get_stream(None);
            assert_eq!(stream.next().await.unwrap(), 0);
        });

        cache.insert(0, 0);
    }
}
