// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! DashMap + Per-Thread Arena with Explicit Thread Indices (Implementation 6).
//!
//! Uses explicit thread indices assigned at thread creation for zero-contention allocations.
//!
//! Characteristics:
//! - Lock-free reads
//! - Zero contention (each thread locks only its own arena)
//! - Explicit index assignment (no dynamic HashMap lookups)
//! - Best write performance (direct array indexing)
//! - Better cache locality (array-based)
//! - Simpler flush (iterate fixed-size array)
//! - Framework-agnostic (works with any thread pool)
//! - Requires stable thread pool (threads don't exit during interner lifetime)
//! - Must set index at thread startup (one-time cost per thread)

use super::{arena::Arena, StablePtr};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::{cell::Cell, hash::Hash, mem};
use crossbeam_utils::CachePadded;

thread_local! {
    /// Thread-local storage for worker index (set once at thread startup).
    static THREAD_INDEX: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Sets the current thread's index (call once per thread at startup).
pub fn set_thread_index(idx: usize) {
    THREAD_INDEX.with(|index| index.set(Some(idx)));
}

/// Gets the current thread's index, returns None if not set.
pub fn get_thread_index() -> Option<usize> {
    THREAD_INDEX.with(|index| index.get())
}

/// DashMap-based interner with per-thread arenas using explicit indices.
pub struct DashMapPerThreadArrayInterner<T: Hash + Eq + Clone + Send + Sync + 'static> {
    map: DashMap<&'static T, StablePtr<T>>,
    arenas: Box<[CachePadded<Mutex<Arena<T>>>]>,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> DashMapPerThreadArrayInterner<T> {
    /// Creates a new interner with the specified number of thread arenas.
    pub fn new(thread_count: usize) -> Self {
        let arenas: Box<[CachePadded<Mutex<Arena<T>>>]> = (0..thread_count)
            .map(|_| CachePadded::new(Mutex::new(Arena::new())))
            .collect();

        Self {
            map: DashMap::new(),
            arenas,
        }
    }

    /// Interns a value with lock-free reads and per-thread allocations.
    ///
    /// # Panics
    ///
    /// Panics if the thread index has not been set via `set_thread_index()`.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Lock-free read
        if let Some(entry) = self.map.get(value) {
            return *entry;
        }

        // Get thread index from TLS (set at thread startup)
        let thread_idx = THREAD_INDEX.with(|idx| {
            idx.get()
                .expect("Thread index not set - did you call set_thread_index?")
        });

        // Direct array indexing - no HashMap lookup!
        let ptr = {
            let mut arena = self.arenas[thread_idx].lock();
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

    /// Returns the number of thread arenas.
    pub fn arena_count(&self) -> usize {
        self.arenas.len()
    }
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> Default for DashMapPerThreadArrayInterner<T> {
    fn default() -> Self {
        // Default to number of CPUs
        Self::new(num_cpus::get())
    }
}

// Note: We don't implement InternerImpl for this type because it requires
// a thread_count parameter that the trait's new() doesn't support.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        set_thread_index(0);
        let interner = DashMapPerThreadArrayInterner::new(1);
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

        let thread_count = 10;
        let interner = Arc::new(DashMapPerThreadArrayInterner::new(thread_count));
        let mut handles = vec![];

        for i in 0..thread_count {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                set_thread_index(i);
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

        let thread_count = 10;
        let interner = Arc::new(DashMapPerThreadArrayInterner::new(thread_count));
        let mut handles = vec![];

        for i in 0..thread_count {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                set_thread_index(i);
                interner.intern(&format!("value_{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(interner.len(), thread_count);
    }

    #[test]
    fn test_flush() {
        set_thread_index(0);
        let mut interner = DashMapPerThreadArrayInterner::new(1);
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Thread index not set")]
    fn test_panics_without_thread_index() {
        // Clear any previously set index
        THREAD_INDEX.with(|idx| idx.set(None));

        let interner = DashMapPerThreadArrayInterner::new(1);
        // This should panic
        interner.intern(&"test");
    }
}
