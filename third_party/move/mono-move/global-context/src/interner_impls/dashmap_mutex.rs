// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! DashMap + Mutex<Arena> interner (Implementation 4).
//!
//! Lock-free reads with DashMap, single-mutex allocations.
//!
//! Characteristics:
//! - Completely lock-free reads
//! - DashMap has 64 internal segments (fine-grained write locks)
//! - Arena mutex is bottleneck on writes
//! - Leaks allocations on lost races

use super::{arena::Arena, InternerImpl, StablePtr};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::{hash::Hash, mem};

/// DashMap-based interner with single mutex-protected arena.
pub struct DashMapMutexInterner<T: Hash + Eq + Clone + Send + Sync + 'static> {
    map: DashMap<&'static T, StablePtr<T>>,
    arena: Mutex<Arena<T>>,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> DashMapMutexInterner<T> {
    /// Creates a new interner.
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            arena: Mutex::new(Arena::new()),
        }
    }

    /// Interns a value with lock-free reads.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Lock-free read
        if let Some(entry) = self.map.get(value) {
            return *entry;
        }

        // Mutex-protected allocation
        let ptr = {
            let mut arena = self.arena.lock();
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
        self.arena.get_mut().flush();
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let arena = self.arena.lock();
        let map_size = self.map.len() * (mem::size_of::<&T>() + mem::size_of::<StablePtr<T>>());
        map_size + arena.memory_usage()
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

impl<T: Hash + Eq + Clone + Send + Sync + 'static> InternerImpl<T> for DashMapMutexInterner<T> {
    fn new() -> Self {
        DashMapMutexInterner::new()
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

impl<T: Hash + Eq + Clone + Send + Sync + 'static> Default for DashMapMutexInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = DashMapMutexInterner::new();
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

        let interner = Arc::new(DashMapMutexInterner::new());
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

        let interner = Arc::new(DashMapMutexInterner::new());
        let mut handles = vec![];

        for i in 0..10 {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                interner.intern(&format!("value_{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(interner.len(), 10);
    }

    #[test]
    fn test_flush() {
        let mut interner = DashMapMutexInterner::new();
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }
}
