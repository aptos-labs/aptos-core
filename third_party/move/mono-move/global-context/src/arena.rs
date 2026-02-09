// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use bumpalo::Bump;
use crossbeam::utils::CachePadded;
use parking_lot::{Mutex, MutexGuard};
use std::ptr::NonNull;

/// Arena allocator API that never releases memory.
pub trait ArenaAllocator {
    /// Allocates a value in the arena, returning a stable pointer to it.
    fn alloc<T>(&self, value: T) -> ArenaPtr<T>;

    /// Allocates a string in the arena, returning a stable pointer to it.
    fn alloc_str(&self, s: &str) -> ArenaPtr<str>;

    /// Allocates a slice by copying from the source slice, returning a stable pointer.
    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> ArenaPtr<[T]>;

    /// Returns the number of allocated bytes in the arena (may include padding, etc.)
    fn allocated_bytes(&self) -> usize;
}

/// A stable pointer allocated by [`ArenaAllocator`]. Guaranteed to be alive
/// unless the arena has been flushed.
///
/// # Safety
///
/// After flushing the arena, dereferencing this pointer is UB.
#[repr(transparent)]
pub struct ArenaPtr<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> ArenaPtr<T> {
    /// Unsafely casts this arena pointer to a reference with the specified lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// 1. The arena has not been flushed since this pointer was created,
    /// 2. The returned reference does not outlive the arena's actual lifetime,
    /// 3. This is called within [`crate::ExecutionContext`] scope (which prevents
    ///    concurrent flush by blocking [`crate::MaintenanceContext`]).
    ///
    /// See the module-level documentation in `context.rs` for a complete safety argument.
    pub(crate) unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY: caller must ensure above-mentioned safety preconditions.
        unsafe { self.0.as_ref() }
    }
}

// SAFETY:
//  This is just a pointer to arena-allocated data. If T is Send, then sending
//  the pointer to another thread is safe because the arena keeps the allocation
//  stable until flush.
unsafe impl<T: ?Sized + Send> Send for ArenaPtr<T> {}

// SAFETY:
//   This is just a pointer to arena-allocated data. If T is Sync, then sharing
//   the pointer across threads is safe because the arena keeps the allocation
//   stable and T can be safely accessed from multiple threads.
unsafe impl<T: ?Sized + Sync> Sync for ArenaPtr<T> {}

impl<T: ?Sized> Copy for ArenaPtr<T> {}

impl<T: ?Sized> Clone for ArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for ArenaPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() as *const () == other.0.as_ptr() as *const ()
    }
}

impl<T: ?Sized> Eq for ArenaPtr<T> {}

impl<T: ?Sized> std::hash::Hash for ArenaPtr<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_ptr() as *const ()).hash(state)
    }
}

/// A pool of bump allocators that never deallocates individual items.
pub struct ArenaPool {
    /// Arenas backing allocations of data.
    arenas: Box<[CachePadded<Mutex<Bump>>]>,
    /// Number of arenas.
    num_arenas: usize,
}

/// A bump allocator borrowed from [`ArenaPool`].
pub struct ArenaGuard<'a> {
    guard: MutexGuard<'a, Bump>,
}

impl ArenaPool {
    /// Creates a pool with a single arena of specified capacity.
    pub fn with_capacity(arena_capacity: usize) -> Self {
        Self::with_capacity_and_num_arenas(arena_capacity, 1)
    }

    /// Creates the specified number of arenas in the pool with 0 capacity.
    pub fn with_num_arenas(num_arenas: usize) -> Self {
        Self::with_capacity_and_num_arenas(0, num_arenas)
    }

    /// Creates the specified number of arenas in the pool, each with the
    /// specified capacity.
    pub fn with_capacity_and_num_arenas(arena_capacity: usize, num_arenas: usize) -> Self {
        // Number of arenas is ~ number of working threads. Upper bound by 128 is good enough to
        // accommodate most of the chips.
        assert!(num_arenas > 0);
        assert!(num_arenas <= 128);
        assert!(num_arenas.is_power_of_two());

        let arenas = (0..num_arenas)
            .map(|_| CachePadded::new(Mutex::new(Bump::with_capacity(arena_capacity))))
            .collect();
        Self { arenas, num_arenas }
    }

    /// Locks the arena at a specific index and returns its guard. Returns
    /// [`None`] if the lock cannot be obtained.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn lock_arena(&self, idx: usize) -> Option<ArenaGuard<'_>> {
        assert!(idx < self.num_arenas);
        Some(ArenaGuard {
            guard: self.arenas[idx].try_lock()?,
        })
    }

    /// Returns the number of arenas in the pool.
    pub fn num_arenas(&self) -> usize {
        self.num_arenas
    }

    /// Returns the number of allocated bytes for a specific arena in the pool.
    pub fn allocated_bytes(&self, idx: usize) -> usize {
        assert!(idx < self.num_arenas);
        self.arenas[idx].lock().allocated_bytes()
    }

    /// Returns the total number of allocated bytes across all arenas in the pool.
    pub fn allocated_bytes_sum(&self) -> usize {
        self.arenas
            .iter()
            .map(|arena| arena.lock().allocated_bytes())
            .sum()
    }

    /// Resets all arenas in the pool, making all allocations invalid.
    ///
    /// # Safety
    ///
    /// The caller must ensure that pointers given out by arena are not alive.
    /// Dereferencing such pointers are flush is UB.
    pub unsafe fn flush(&self) {
        for arena in self.arenas.iter() {
            arena.lock().reset();
        }
    }
}

impl Default for ArenaPool {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl ArenaAllocator for ArenaGuard<'_> {
    fn alloc<T>(&self, value: T) -> ArenaPtr<T> {
        ArenaPtr(NonNull::from(self.guard.alloc(value)))
    }

    fn alloc_str(&self, s: &str) -> ArenaPtr<str> {
        ArenaPtr(NonNull::from(self.guard.alloc_str(s)))
    }

    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> ArenaPtr<[T]> {
        let slice_ref = self.guard.alloc_slice_copy(src);
        ArenaPtr(NonNull::from(slice_ref))
    }

    fn allocated_bytes(&self) -> usize {
        self.guard.allocated_bytes()
    }
}
