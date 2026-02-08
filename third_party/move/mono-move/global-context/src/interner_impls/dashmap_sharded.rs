// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! DashMap + Sharded Arena interner (Implementation 5).
//!
//! Lock-free reads with sharded allocations (64 arenas).
//!
//! Characteristics:
//! - Lock-free reads
//! - 64 independent arenas (minimal contention)
//! - Scales well to 64+ cores
//! - 64× memory overhead (each arena allocates independently)

use super::{arena::Arena, InternerImpl, StablePtr};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::{
    hash::{Hash, Hasher},
    mem,
};
use crossbeam_utils::CachePadded;
use ahash::RandomState;

const SHARD_COUNT: usize = 64;

/// DashMap-based interner with sharded arenas.
pub struct DashMapShardedInterner<T: Hash + Eq + Clone + Send + Sync + 'static> {
    map: DashMap<&'static T, StablePtr<T>>,
    arenas: Box<[CachePadded<Mutex<Arena<T>>>]>,
    shard_mask: usize,
    hasher: RandomState,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> DashMapShardedInterner<T> {
    /// Creates a new interner with 64 sharded arenas.
    pub fn new() -> Self {
        let arenas: Box<[CachePadded<Mutex<Arena<T>>>]> =
            (0..SHARD_COUNT).map(|_| CachePadded::new(Mutex::new(Arena::new()))).collect();

        Self {
            map: DashMap::new(),
            arenas,
            shard_mask: SHARD_COUNT - 1,
            hasher: RandomState::default(),
        }
    }

    /// Selects an arena based on value's hash.
    fn arena_for(&self, value: &T) -> &Mutex<Arena<T>> {
        let hash = self.hasher.hash_one(value);
        let idx = (hash as usize) & self.shard_mask;
        &self.arenas[idx]
    }

    /// Interns a value with lock-free reads.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Lock-free read
        if let Some(entry) = self.map.get(value) {
            return *entry;
        }

        // Allocate in sharded arena
        let ptr = {
            let mut arena = self.arena_for(value).lock();
            unsafe { arena.alloc(value.clone()) }
        };

        // Insert into DashMap (fine-grained locking)
        // SAFETY: ptr is stable and points to valid data in the arena
        let key: &'static T = unsafe { &*ptr.as_ptr() };
        *self.map.entry(key).or_insert(ptr)
    }

    /// Clears all interned values.
    pub fn flush(&mut self) {
        self.map.clear();
        for arena in self.arenas.iter_mut() {
            arena.get_mut().flush();
        }
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let map_size = self.map.len() * (mem::size_of::<&T>() + mem::size_of::<StablePtr<T>>());
        let arenas_size: usize = self
            .arenas
            .iter()
            .map(|arena| arena.lock().memory_usage())
            .sum();
        map_size + arenas_size
    }

    /// Returns the number of unique interned values.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if no values are interned.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> InternerImpl<T> for DashMapShardedInterner<T> {
    fn new() -> Self {
        DashMapShardedInterner::new()
    }

    fn intern(&self, value: &T) -> StablePtr<T> {
        self.intern(value)
    }

    fn flush(&mut self) {
        self.flush()
    }

    fn memory_usage(&self) -> usize {
        self.memory_usage()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> Default for DashMapShardedInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = DashMapShardedInterner::new();
        let ptr1 = interner.intern(&"hello");
        let ptr2 = interner.intern(&"world");
        let ptr3 = interner.intern(&"hello");

        // Same value should return same pointer
        assert_eq!(ptr1, ptr3);
        assert_ne!(ptr1, ptr2);

        unsafe {
            assert_eq!(*ptr1.as_ref(), "hello");
            assert_eq!(*ptr2.as_ref(), "world");
        }

        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_concurrent_same_value() {
        use std::{sync::Arc, thread};

        let interner = Arc::new(DashMapShardedInterner::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                // Just intern the value, don't return the pointer
                interner.intern(&"shared");
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All threads should have interned the same value
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_concurrent_different_values() {
        use std::{sync::Arc, thread};

        let interner = Arc::new(DashMapShardedInterner::new());
        let mut handles = vec![];

        for i in 0..100 {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                interner.intern(&format!("value_{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(interner.len(), 100);
    }

    #[test]
    fn test_flush() {
        let mut interner = DashMapShardedInterner::new();
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }
}
