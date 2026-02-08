// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! RwLock<BTreeMap + Arena> interner (Implementation 1).
//!
//! Baseline implementation with everything under a single RwLock.
//! Uses BTreeMap for deterministic iteration order (useful for debugging).
//!
//! Characteristics:
//! - Simple, proven pattern
//! - Deterministic iteration
//! - Write lock blocks ALL reads
//! - Write lock blocks ALL allocations
//! - O(log n) lookup
//! - Poor scalability

use super::{arena::Arena, InternerImpl, StablePtr};
use parking_lot::RwLock;
use std::{collections::BTreeMap, hash::Hash, mem};

/// Inner state protected by RwLock.
struct Inner<T: Ord + Send + Sync + 'static> {
    map: BTreeMap<&'static T, StablePtr<T>>,
    arena: Arena<T>,
}

/// RwLock-based interner with BTreeMap for deterministic ordering.
pub struct RwLockBTreeInterner<T: Ord + Clone + Send + Sync + 'static> {
    inner: RwLock<Inner<T>>,
}

impl<T: Ord + Clone + Send + Sync + 'static> RwLockBTreeInterner<T> {
    /// Creates a new interner.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                map: BTreeMap::new(),
                arena: Arena::new(),
            }),
        }
    }

    /// Interns a value with double-checked locking pattern.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Fast path: read lock for lookup
        {
            let inner = self.inner.read();
            if let Some(&ptr) = inner.map.get(value) {
                return ptr;
            }
        }

        // Slow path: write lock for allocation and insertion
        let mut inner = self.inner.write();

        // Double-check (another thread might have inserted while we waited)
        if let Some(&ptr) = inner.map.get(value) {
            return ptr;
        }

        // Allocate and insert under write lock
        let ptr = unsafe { inner.arena.alloc(value.clone()) };
        // SAFETY: ptr is stable and points to valid data in the arena
        let key: &'static T = unsafe { &*ptr.as_ptr() };
        inner.map.insert(key, ptr);
        ptr
    }

    /// Clears all interned values.
    pub fn flush(&mut self) {
        let inner = self.inner.get_mut();
        inner.map.clear();
        inner.arena.flush();
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let inner = self.inner.read();
        let map_size = inner.map.len() * (mem::size_of::<&T>() + mem::size_of::<StablePtr<T>>());
        map_size + inner.arena.memory_usage()
    }

    /// Returns the number of unique interned values.
    pub fn len(&self) -> usize {
        self.inner.read().map.len()
    }

    /// Returns true if no values are interned.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Hash + Eq + Ord + Clone + Send + Sync + 'static> InternerImpl<T>
    for RwLockBTreeInterner<T>
{
    fn new() -> Self {
        RwLockBTreeInterner::new()
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

impl<T: Ord + Clone + Send + Sync + 'static> Default for RwLockBTreeInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = RwLockBTreeInterner::new();
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

        let interner = Arc::new(RwLockBTreeInterner::new());
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
    fn test_flush() {
        let mut interner = RwLockBTreeInterner::new();
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }
}
