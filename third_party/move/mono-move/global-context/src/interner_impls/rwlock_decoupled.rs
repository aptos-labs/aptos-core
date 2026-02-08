// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! RwLock<HashMap> + Mutex<Arena> interner (Implementation 3).
//!
//! Decoupled design where map and arena have independent locks.
//!
//! Characteristics:
//! - Map operations don't block allocations
//! - Allocations don't block reads (mostly)
//! - Still have write lock on map
//! - Can leak allocations on lost races

use super::{arena::Arena, InternerImpl, StablePtr};
use parking_lot::{Mutex, RwLock};
use std::{collections::HashMap, hash::Hash, mem};

/// Decoupled interner with separate locks for map and arena.
pub struct RwLockDecoupledInterner<T: Hash + Eq + Clone + Send + Sync + 'static> {
    map: RwLock<HashMap<&'static T, StablePtr<T>>>,
    arena: Mutex<Arena<T>>,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> RwLockDecoupledInterner<T> {
    /// Creates a new interner.
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
            arena: Mutex::new(Arena::new()),
        }
    }

    /// Interns a value, allocating before acquiring write lock.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Fast path: read lock for lookup
        {
            let map = self.map.read();
            if let Some(&ptr) = map.get(value) {
                return ptr;
            }
        }

        // Allocate BEFORE acquiring write lock on map
        // This reduces critical section duration
        let ptr = {
            let mut arena = self.arena.lock();
            unsafe { arena.alloc(value.clone()) }
        };

        // Write lock only for map insertion
        let mut map = self.map.write();

        // Double-check (race condition: another thread might have inserted)
        if let Some(&existing) = map.get(value) {
            // Lost race, our allocation is leaked (acceptable tradeoff)
            return existing;
        }

        // SAFETY: ptr is stable and points to valid data in the arena
        let key: &'static T = unsafe { &*ptr.as_ptr() };
        map.insert(key, ptr);
        ptr
    }

    /// Clears all interned values.
    pub fn flush(&mut self) {
        self.map.get_mut().clear();
        self.arena.get_mut().flush();
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let map = self.map.read();
        let arena = self.arena.lock();
        let map_size = map.len() * (mem::size_of::<&T>() + mem::size_of::<StablePtr<T>>());
        map_size + arena.memory_usage()
    }

    /// Returns the number of unique interned values.
    pub fn len(&self) -> usize {
        self.map.read().len()
    }

    /// Returns true if no values are interned.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> InternerImpl<T> for RwLockDecoupledInterner<T> {
    fn new() -> Self {
        RwLockDecoupledInterner::new()
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

impl<T: Hash + Eq + Clone + Send + Sync + 'static> Default for RwLockDecoupledInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = RwLockDecoupledInterner::new();
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

        let interner = Arc::new(RwLockDecoupledInterner::new());
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

        // All threads should have interned the same value (though some allocations may be leaked)
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_flush() {
        let mut interner = RwLockDecoupledInterner::new();
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }
}
