// Copyright © Aptos Foundation

use dashmap::DashMap;
use rayon::prelude::*;
use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};

pub type CompressedKey = u32;

pub struct ParallelKeyCompressor<K> {
    key_mapping: DashMap<K, CompressedKey>,
    // TODO: consider using thread-local counters instead of atomic.
    next_key: AtomicU32,
}

impl<K: Hash + Eq> ParallelKeyCompressor<K> {
    pub fn new() -> Self {
        Self {
            key_mapping: DashMap::new(),
            next_key: AtomicU32::new(0),
        }
    }
}

impl<K: Hash + Eq> Default for ParallelKeyCompressor<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Clone + Eq + Send + Sync> ParallelKeyCompressor<K> {
    pub fn map_key(&self, key: &K) -> CompressedKey {
        if let Some(entry) = self.key_mapping.get(key) {
            *entry.value()
        } else {
            let mapped_key = self.next_key.fetch_add(1, SeqCst);
            // The resulting key may be different from `mapped_key` due to concurrency.
            // This may create "holes" in the key space, but that's fine.
            *self
                .key_mapping
                .entry(key.clone())
                .or_insert(mapped_key)
                .value()
        }
    }
}
