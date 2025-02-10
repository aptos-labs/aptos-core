// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{SizedCache, SizedCacheEntry};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

type CacheEntry<T> = SizedCacheEntry<usize, T>;
type CacheEntryLock<T> = Mutex<Option<CacheEntry<T>>>;

const DEFAULT_MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

/// An in-memory cache that uses a mutex to synchronize access to the cache entries.
/// The cache is designed with indexing use cases in mind and is backed by a fixed-size array.
///
/// ## Example
///
/// A cache that supports out of order insertion and a deterministic eviction policy that evicts
/// the smallest value.
///
/// 1. Initialize cache with a capacity of 9 and eviction limit of 5
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// |   |   |   |   |   |   |   |   |   |
/// +---+---+---+---+---+---+---+---+---+
/// ```
///
/// 2. Insert 5 → `5 % 9 = 5` → `cache[5] = 5`
/// * Capacity at 1
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// |   |   |   |   |   | 5 |   |   |   |
/// +---+---+---+---+---+---+---+---+---+
/// ```
///
/// 3. Insert 6, 7, 8
/// * Capacity at 4
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// |   |   |   |   |   | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+---+
/// ```
///
/// 4. Insert 9 → `9 % 9 = 0` → `cache[0] = 9`
/// * Capacity at 5 (limit)
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// | 9 |   |   |   |   | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+---+
/// ```
///
/// 5. Insert 11 → `11 % 9 = 2` → `cache[2] = 11`  
///    Evict 5 → `5 % 9 = 5` → `cache[5] = None`
/// * Capacity at 5
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// | 9 |   | 11|   |   |   | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+---+
/// ```
///
/// 6. Insert 10 → `10 % 9 = 1` → `cache[1] = 10`  
///    Evict 6 → `6 % 9 = 6` → `cache[6] = None`
/// * Capacity at 5
/// ```text
/// +---+---+---+---+---+---+---+---+---+
/// | 9 | 10| 11|   |   |   |   | 7 | 8 |
/// +---+---+---+---+---+---+---+---+---+
/// ```
#[derive(Debug)]
pub struct SyncMutexCache<T: Send + Sync + Clone> {
    cache: Box<[CacheEntryLock<T>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> SyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    /// Initializes a new [`SyncMutexCache`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Mutex::new(None));
        }

        Self {
            cache: buffer.into_boxed_slice(),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for SyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_NUM_CACHE_ITEMS)
    }
}

impl<T> SizedCache<usize, T> for SyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    fn get(&self, key: &usize) -> Option<CacheEntry<T>> {
        let index = *key % self.capacity;
        let lock = self.cache[index].lock();
        lock.clone()
    }

    fn insert_with_size(&self, key: usize, value: T, size_in_bytes: usize) -> usize {
        let index = key % self.capacity;
        let mut lock = self.cache[index].lock();

        // Update cache size
        if let Some(prev) = &*lock {
            self.size.fetch_sub(prev.size_in_bytes, Ordering::Relaxed);
        }

        // Update cache entry
        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);
        *lock = Some(SizedCacheEntry {
            key,
            value,
            size_in_bytes,
        });

        index
    }

    fn evict(&self, key: &usize) -> Option<CacheEntry<T>> {
        let index = *key % self.capacity;
        let mut lock = self.cache[index].lock();

        // Update the cache size and set the value at key to none
        if let Some(prev) = lock.take() {
            self.size.fetch_sub(prev.size_in_bytes, Ordering::Relaxed);
            return Some(prev);
        }
        None
    }

    fn total_size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}
