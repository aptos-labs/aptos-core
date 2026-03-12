// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! # Global arena
//!
//! [`GlobalArenaPool`] is used for bump-allocating long-lived data (types,
//! identifiers, etc.) that outlives executables and is not subject to code
//! upgrade invalidation. Returns [`GlobalArenaPtr<T>`] which is a raw pointer
//! to arena's allocation.
//!
//! ## Safety model
//!
//! [`GlobalArenaPtr<T>`] exposes **only immutable access** to the allocated
//! data, and it is sound to share the pointers across threads. Dereferencing
//! the pointer is a separate concern captured in two **unsafe** contracts:
//!
//! - [`GlobalArenaPtr::as_ref_unchecked`] — caller must ensure that the arena
//!   that owns the data has not been reset or dropped, and that the pointer
//!   has not been invalidated.
//! - [`GlobalArenaPool::reset_unchecked`] — caller must ensure there are no
//!   live pointers derived from the allocations that is about to be reset.

use bumpalo::Bump;
use crossbeam_utils::CachePadded;
use parking_lot::{Mutex, MutexGuard};
use std::{
    hash::{Hash, Hasher},
    ptr::NonNull,
};

/// A non-null pointer into a [`GlobalArenaShard`] allocation.
#[repr(transparent)]
pub struct GlobalArenaPtr<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> GlobalArenaPtr<T> {
    /// Creates a [`GlobalArenaPtr`] that points to statically allocated data.
    ///
    /// Unlike arena-allocated pointers, the result is never invalidated by
    /// arena reset or arena drop: the static data lives for the entire
    /// lifetime of a program.
    pub fn from_static(data: &'static T) -> Self {
        GlobalArenaPtr(NonNull::from(data))
    }

    /// Unsafely casts this arena pointer to a reference with the specified
    /// lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the arena where data is allocated has not
    /// been reset or dropped while this reference was created. In other words,
    /// the returned reference **must not** outlive the actual lifetime of the
    /// allocation.
    pub unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY:
        //
        // Caller must ensure safety preconditions.
        unsafe { self.0.as_ref() }
    }

    /// Returns the raw const pointer to the allocated data.
    pub fn as_raw_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
}

// SAFETY:
//
// Global pointer only exposes immutable access to pointee type. It is `Send`
// because we are only moving a read-only handle to a pointee type which is
// also `Send` across threads. It also requires pointee to be `Sync` because
// the pointer can be copied, so multiple threads can obtain a reference to
// same pointee (and that requires`Sync`).
unsafe impl<T: ?Sized + Send + Sync> Send for GlobalArenaPtr<T> {}

// SAFETY:
//
// Global pointer only exposes immutable access to pointee type. It is `Sync`
// because sharing it gives concurrent read-only access to pointee type, which
// is safe when pointee is also `Sync`.
unsafe impl<T: ?Sized + Sync> Sync for GlobalArenaPtr<T> {}

impl<T: ?Sized> Copy for GlobalArenaPtr<T> {}

impl<T: ?Sized> Clone for GlobalArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implement pointer-based equality so we can use pointers as keys in data
// structures and also provide a fast equality check.
impl<T: ?Sized> PartialEq for GlobalArenaPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.as_raw_ptr(), other.as_raw_ptr())
    }
}

impl<T: ?Sized> Eq for GlobalArenaPtr<T> {}

// Implements pointer-based hash so we can use pointers as keys in hash-based
// data structures.
impl<T: ?Sized> Hash for GlobalArenaPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_raw_ptr().hash(state)
    }
}

/// A pool of bump arenas that never deallocate individual items. Each arena
/// from the pool can be acquired by the running thread to obtain an exclusive
/// access.
pub struct GlobalArenaPool {
    arenas: Box<[CachePadded<Mutex<Bump>>]>,
    num_arenas: usize,
}

impl GlobalArenaPool {
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
    ///
    /// # Panics
    ///
    /// - If number of arenas is zero, or larger than 128.
    /// - If number of arenas is not a power of two.
    pub fn with_capacity_and_num_arenas(arena_capacity: usize, num_arenas: usize) -> Self {
        // Number of arenas is ~ number of working threads. Upper bound by 128
        // is good enough to accommodate most of the CPUs.
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
    pub fn try_lock_arena(&self, idx: usize) -> Option<GlobalArenaShard<'_>> {
        assert!(idx < self.num_arenas);
        Some(GlobalArenaShard {
            guard: self.arenas[idx].try_lock()?,
        })
    }

    /// Locks the arena at a specific index and returns its guard. Blocks if
    /// the arena is currently being acquired by other thread.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn lock_arena(&self, idx: usize) -> GlobalArenaShard<'_> {
        assert!(idx < self.num_arenas);
        GlobalArenaShard {
            guard: self.arenas[idx].lock(),
        }
    }

    /// Returns the number of arenas in the pool.
    pub fn num_arenas(&self) -> usize {
        self.num_arenas
    }

    /// Returns the number of allocated bytes for a specific arena in the pool.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn allocated_bytes(&self, idx: usize) -> usize {
        assert!(idx < self.num_arenas);
        self.arenas[idx].lock().allocated_bytes()
    }

    /// Resets all arenas in the pool, making **all** allocations invalid.
    ///
    /// # Safety
    ///
    /// The caller **must** ensure there are no live pointers pointing to the
    /// data allocated in the arena that is about to be cleared.
    pub unsafe fn reset_unchecked(&self) {
        for arena in self.arenas.iter() {
            arena.lock().reset();
        }
    }
}

impl Default for GlobalArenaPool {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

/// A bump allocator borrowed from [`GlobalArenaPool`].
pub struct GlobalArenaShard<'a> {
    guard: MutexGuard<'a, Bump>,
}

impl<'a> GlobalArenaShard<'a> {
    /// Allocates a value in the arena, returning a stable pointer to it.
    pub fn alloc<T>(&self, value: T) -> GlobalArenaPtr<T> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc(value)))
    }

    /// Allocates a string in the arena, returning a stable pointer to it.
    pub fn alloc_str(&self, s: &str) -> GlobalArenaPtr<str> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc_str(s)))
    }

    /// Allocates a slice by copying from the source, returning a stable pointer.
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> GlobalArenaPtr<[T]> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc_slice_copy(src)))
    }
}
