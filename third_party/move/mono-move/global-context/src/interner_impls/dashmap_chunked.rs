// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! DashMap + Chunked Arena interner (Implementation 7).
//!
//! Advanced implementation with pre-allocated chunks and atomic index allocation.
//!
//! Characteristics:
//! - Lock-free reads
//! - Truly concurrent writes (atomic index)
//! - Only locks on chunk exhaustion (rare)
//! - Complex implementation
//! - Wasted memory (pre-allocated slots)
//! - Chunk swap contention when buffer fills

use super::{InternerImpl, StablePtr};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use parking_lot::Mutex;
use std::{
    hash::Hash,
    mem::{self, MaybeUninit},
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// Cache-padded atomic to avoid false sharing.
#[repr(align(64))]
struct CachePadded<T>(T);

impl<T> CachePadded<T> {
    fn new(value: T) -> Self {
        CachePadded(value)
    }
}

impl<T> std::ops::Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A chunk of pre-allocated memory for concurrent allocation.
struct Chunk<T> {
    buffer: Box<[MaybeUninit<T>]>,
    index: CachePadded<AtomicUsize>,
    capacity: usize,
}

impl<T> Chunk<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: (0..capacity).map(|_| MaybeUninit::uninit()).collect(),
            index: CachePadded::new(AtomicUsize::new(0)),
            capacity,
        }
    }

    /// Tries to allocate in this chunk atomically.
    ///
    /// Returns None if the chunk is full.
    unsafe fn try_alloc(&self, value: T) -> Option<StablePtr<T>> {
        let idx = self.index.fetch_add(1, Ordering::AcqRel);

        if idx < self.capacity {
            // Success! Write to pre-allocated slot
            let slot = &self.buffer[idx];
            let ptr = slot.as_ptr() as *mut T;
            unsafe {
                ptr.write(value);
                let non_null = NonNull::new_unchecked(ptr);
                Some(StablePtr::new(non_null))
            }
        } else {
            None
        }
    }
}

/// Chunked arena with atomic allocation.
struct ChunkedArena<T: Clone + Send + Sync> {
    current: ArcSwap<Chunk<T>>,
    frozen_chunks: Mutex<Vec<Arc<Chunk<T>>>>,
    next_size: CachePadded<AtomicUsize>,
}

impl<T: Clone + Send + Sync> ChunkedArena<T> {
    const INITIAL_CAPACITY: usize = 256;

    fn new() -> Self {
        Self {
            current: ArcSwap::new(Arc::new(Chunk::new(Self::INITIAL_CAPACITY))),
            frozen_chunks: Mutex::new(Vec::new()),
            next_size: CachePadded::new(AtomicUsize::new(Self::INITIAL_CAPACITY * 2)),
        }
    }

    unsafe fn alloc(&self, value: T) -> StablePtr<T> {
        loop {
            let current = self.current.load();

            // Try atomic allocation
            // SAFETY: try_alloc is unsafe but we own the data and synchronization is correct
            if let Some(ptr) = unsafe { current.try_alloc(value.clone()) } {
                return ptr;
            }

            // Chunk full, allocate new chunk (rare, requires lock)
            let next_size = self.next_size.load(Ordering::Acquire);
            let new_chunk = Arc::new(Chunk::new(next_size));

            // Swap in the new chunk
            let old = self.current.swap(new_chunk);

            // Freeze old chunk by keeping an Arc reference
            {
                let mut frozen = self.frozen_chunks.lock();
                // Keep the Arc alive so the allocations remain valid
                frozen.push(old);
            }

            self.next_size
                .store(next_size.saturating_mul(2), Ordering::Release);

            // Retry with new chunk
        }
    }

    #[allow(dead_code)]
    fn len(&self) -> usize {
        let current = self.current.load();
        let current_count = current.index.load(Ordering::Acquire).min(current.capacity);

        let frozen = self.frozen_chunks.lock();
        let frozen_count: usize = frozen
            .iter()
            .map(|chunk| chunk.index.load(Ordering::Acquire).min(chunk.capacity))
            .sum();

        current_count + frozen_count
    }

    fn memory_usage(&self) -> usize {
        let current = self.current.load();
        let current_size = current.capacity * mem::size_of::<T>();

        let frozen = self.frozen_chunks.lock();
        let frozen_size: usize = frozen
            .iter()
            .map(|chunk| chunk.capacity * mem::size_of::<T>())
            .sum();

        current_size + frozen_size
    }

    fn flush(&mut self) {
        self.current = ArcSwap::new(Arc::new(Chunk::new(Self::INITIAL_CAPACITY)));
        self.frozen_chunks.get_mut().clear();
        self.next_size = CachePadded::new(AtomicUsize::new(Self::INITIAL_CAPACITY * 2));
    }
}

/// DashMap-based interner with chunked arena for lock-free allocations.
pub struct DashMapChunkedInterner<T: Hash + Eq + Clone + Send + Sync + 'static> {
    map: DashMap<&'static T, StablePtr<T>>,
    arena: ChunkedArena<T>,
}

impl<T: Hash + Eq + Clone + Send + Sync + 'static> DashMapChunkedInterner<T> {
    /// Creates a new interner.
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            arena: ChunkedArena::new(),
        }
    }

    /// Interns a value with lock-free reads and mostly lock-free writes.
    pub fn intern(&self, value: &T) -> StablePtr<T> {
        // Lock-free read
        if let Some(entry) = self.map.get(value) {
            return *entry;
        }

        // Atomic allocation (lock-free unless chunk is exhausted)
        let ptr = unsafe { self.arena.alloc(value.clone()) };

        // Insert into DashMap (fine-grained locking)
        // SAFETY: ptr is stable and points to valid data in the arena
        let key: &'static T = unsafe { &*ptr.as_ptr() };
        *self.map.entry(key).or_insert(ptr)
    }

    /// Clears all interned values.
    pub fn flush(&mut self) {
        self.map.clear();
        self.arena.flush();
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let map_size = self.map.len() * (mem::size_of::<&T>() + mem::size_of::<StablePtr<T>>());
        map_size + self.arena.memory_usage()
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

impl<T: Hash + Eq + Clone + Send + Sync + 'static> InternerImpl<T> for DashMapChunkedInterner<T> {
    fn new() -> Self {
        DashMapChunkedInterner::new()
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

impl<T: Hash + Eq + Clone + Send + Sync + 'static> Default for DashMapChunkedInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = DashMapChunkedInterner::new();
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

        let interner = Arc::new(DashMapChunkedInterner::new());
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

        let interner = Arc::new(DashMapChunkedInterner::new());
        let mut handles = vec![];

        for i in 0..1000 {
            let interner = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                interner.intern(&format!("value_{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(interner.len(), 1000);
    }

    #[test]
    fn test_flush() {
        let mut interner = DashMapChunkedInterner::new();
        interner.intern(&42);
        interner.intern(&100);
        assert_eq!(interner.len(), 2);

        interner.flush();
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn test_chunk_growth() {
        let interner = DashMapChunkedInterner::new();

        // Allocate many values to trigger chunk growth
        for i in 0..10000 {
            interner.intern(&i);
        }

        assert_eq!(interner.len(), 10000);
    }
}
