use crate::{SizedCache, SizedCacheEntry};
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

const MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

pub struct SyncMutexCache<T: Send + Sync + Clone> {
    cache: Box<[Arc<Mutex<Option<SizedCacheEntry<T>>>>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> SyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Arc::new(Mutex::new(None)));
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
        Self::with_capacity(MAX_NUM_CACHE_ITEMS)
    }
}

impl<T> SizedCache<T> for SyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    fn get(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        let arc = self.cache[*key % self.capacity].clone();
        let lock = arc.lock();
        (*lock).clone()
    }

    fn insert_with_size(&self, key: usize, value: T, size_in_bytes: usize) -> usize {
        // Get lock for cache entry
        let arc = self.cache[key % self.capacity].clone();
        let mut lock = arc.lock();

        // Update cache size
        if let Some(prev_value) = &*lock {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
        }

        // Update cache entry
        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);
        *lock = Some(SizedCacheEntry {
            key,
            value,
            size_in_bytes,
        });

        return key % self.capacity;
    }

    fn evict(&self, key: &usize) {
        // Get lock for cache entry
        let arc = self.cache[*key % self.capacity].clone();
        let mut lock = arc.lock();

        // Update cache size
        if let Some(prev_value) = &*lock {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
        }

        // Set cache entry to None
        *lock = None;
    }

    fn total_size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}
