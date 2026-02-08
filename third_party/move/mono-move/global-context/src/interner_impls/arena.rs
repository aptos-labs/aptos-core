// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Arena allocator for interner implementations.
//!
//! The arena uses a pool of exponentially-growing buffers to provide
//! stable pointers without individual heap allocations.

use super::StablePtr;
use std::{mem, ptr::NonNull};

/// A simple bump allocator that never deallocates individual items.
///
/// Uses exponential growth: 256, 512, 1024, 2048, ... elements per buffer.
#[derive(Debug)]
pub struct Arena<T> {
    /// Current active buffer.
    buffer: Vec<T>,
    /// Pool of previous buffers (kept alive for stable pointers).
    pool: Vec<Vec<T>>,
    /// Next buffer capacity (doubles each time).
    next_size: usize,
}

impl<T> Arena<T> {
    const INITIAL_CAPACITY: usize = 256;

    /// Creates a new empty arena.
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(Self::INITIAL_CAPACITY),
            pool: Vec::new(),
            next_size: Self::INITIAL_CAPACITY * 2,
        }
    }

    /// Allocates a value in the arena and returns a stable pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid as long as the arena is not flushed.
    /// The caller must ensure the pointer is not used after flush().
    pub unsafe fn alloc(&mut self, value: T) -> StablePtr<T> {
        // Check if we need to allocate a new buffer
        if self.buffer.len() >= self.buffer.capacity() {
            let new_buffer = Vec::with_capacity(self.next_size);
            self.next_size = self.next_size.saturating_mul(2);
            let old = mem::replace(&mut self.buffer, new_buffer);
            self.pool.push(old);
        }

        self.buffer.push(value);
        // SAFETY: We just pushed a value, so last() is Some and the pointer is valid.
        // The pointer remains stable because Vec never moves its elements.
        let non_null =
            unsafe { NonNull::new_unchecked(self.buffer.last().unwrap() as *const T as *mut T) };
        unsafe { StablePtr::new(non_null) }
    }

    /// Returns the total number of allocated items.
    pub fn len(&self) -> usize {
        self.pool.iter().map(|buf| buf.len()).sum::<usize>() + self.buffer.len()
    }

    /// Returns true if the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all allocations.
    ///
    /// # Safety
    ///
    /// After calling flush, all previously returned pointers are invalidated.
    pub fn flush(&mut self) {
        self.buffer.clear();
        self.pool.clear();
        self.next_size = Self::INITIAL_CAPACITY * 2;
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let buffer_capacity = self.buffer.capacity() * mem::size_of::<T>();
        let pool_capacity: usize = self
            .pool
            .iter()
            .map(|buf| buf.capacity() * mem::size_of::<T>())
            .sum();
        buffer_capacity + pool_capacity
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_basic() {
        let mut arena = Arena::new();
        let ptr1 = unsafe { arena.alloc(42) };
        let ptr2 = unsafe { arena.alloc(100) };

        unsafe {
            assert_eq!(*ptr1.as_non_null().as_ref(), 42);
            assert_eq!(*ptr2.as_non_null().as_ref(), 100);
        }
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn test_arena_growth() {
        let mut arena = Arena::<u64>::new();

        // Allocate more than initial capacity
        for i in 0..1000 {
            let ptr = unsafe { arena.alloc(i) };
            unsafe {
                assert_eq!(*ptr.as_non_null().as_ref(), i);
            }
        }

        assert_eq!(arena.len(), 1000);
    }

    #[test]
    fn test_arena_flush() {
        let mut arena = Arena::new();
        unsafe {
            arena.alloc(1);
            arena.alloc(2);
            arena.alloc(3);
        }
        assert_eq!(arena.len(), 3);

        arena.flush();
        assert_eq!(arena.len(), 0);
    }
}
