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
//! - [`GlobalArenaPool::reset_all_arenas_unchecked`] — caller must ensure
//!   there are no live pointers derived from the allocations that is about to
//!   be reset and that it is called from single-threaded context (exclusive
//!   access required).
//!
//! # Other pointer types
//!
//! - [`LeakedBoxPtr<T>`] — stable heap pointer from a leaked [`Box`]. Used
//!   for executables in the cache whose address must remain fixed across
//!   concurrent reads. Lifetime is managed manually: freed during the
//!   maintenance phase via [`LeakedBoxPtr::free_unchecked`].

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
    pub unsafe fn as_ref_unchecked<'pool>(&self) -> &'pool T {
        // SAFETY:
        //
        // Caller must ensure safety preconditions.
        unsafe { self.0.as_ref() }
    }

    /// Returns the inner non-null pointer to the allocated data.
    pub fn into_inner(self) -> NonNull<T> {
        self.0
    }

    /// Returns the raw const pointer to the allocated data.
    pub fn as_raw_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
}

// SAFETY:
//
// Global pointer acts as a shared reference when send to other threads. It is
// allocated in the arena, which is guaranteed to be alive throughout the
// lifetime of multiple threads executing. Hence, there is no need to require
// T to be `Send`. However, T has to be `Sync` because global pointer does
// expose a shared reference to T.
unsafe impl<T: ?Sized + Sync> Send for GlobalArenaPtr<T> {}

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
    // Note: use cache-padded to avoid false sharing.
    arenas: Box<[CachePadded<Mutex<Bump>>]>,
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
    pub fn with_capacity_and_num_arenas(arena_capacity: usize, num_arenas: usize) -> Self {
        // Number of arenas is ~ number of working threads. Upper bound by 128
        // is good enough to accommodate most of the CPUs.
        assert!(num_arenas > 0);
        assert!(num_arenas <= 128);

        let arenas = (0..num_arenas)
            .map(|_| CachePadded::new(Mutex::new(Bump::with_capacity(arena_capacity))))
            .collect();
        Self { arenas }
    }

    /// Locks the arena at a specific index and returns its guard. Returns
    /// [`None`] if the lock cannot be obtained.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn try_lock_arena(&self, idx: usize) -> Option<GlobalArenaShard<'_>> {
        assert!(idx < self.num_arenas());
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
        assert!(idx < self.num_arenas());
        GlobalArenaShard {
            guard: self.arenas[idx].lock(),
        }
    }

    /// Returns the number of arenas in the pool.
    pub fn num_arenas(&self) -> usize {
        self.arenas.len()
    }

    /// Returns the number of allocated bytes for a specific arena in the pool.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn allocated_bytes(&self, idx: usize) -> usize {
        assert!(idx < self.num_arenas());
        self.arenas[idx].lock().allocated_bytes()
    }

    /// Resets all arenas in the pool, making **all** allocations invalid.
    ///
    /// # Safety
    ///
    /// 1. The caller **must** ensure there are no live pointers pointing to
    ///    the data allocated in the arena that is about to be cleared.
    /// 2. During iteration, arenas are not locked at the same time. The caller
    ///    **must** ensure that the access is exclusive and there are no race
    ///    conditions.
    // TODO: Consider using &mut to enforce exclusive access at compile-time.
    pub unsafe fn reset_all_arenas_unchecked(&self) {
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
pub struct GlobalArenaShard<'pool> {
    guard: MutexGuard<'pool, Bump>,
}

impl<'pool> GlobalArenaShard<'pool> {
    /// Allocates a value in the arena, returning a raw pointer to it.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the value fails.
    pub fn alloc<T>(&self, value: T) -> GlobalArenaPtr<T> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc(value)))
    }

    /// Allocates a string in the arena, returning a raw pointer to it.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the string fails.
    pub fn alloc_str(&self, s: &str) -> GlobalArenaPtr<str> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc_str(s)))
    }

    /// Allocates a slice by copying from the source, returning a raw pointer.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the slice fails.
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> GlobalArenaPtr<[T]> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc_slice_copy(src)))
    }
}

/// A stable pointer obtained by leaking a [`Box`].
///
/// Used for data that must remain at a fixed address (e.g., executables stored
/// in the cache) but whose lifetime is managed manually. The pointer is valid
/// until [`LeakedBoxPtr::free_unchecked`] is called.
///
/// # Safety model
///
/// The pointer is created by leaking a [`Box`], ensuring a stable heap address.
/// Freeing is **unsafe** — the caller must guarantee that no other references to
/// the data exist.
#[repr(transparent)]
pub struct LeakedBoxPtr<T>(NonNull<T>);

impl<T> LeakedBoxPtr<T> {
    /// Leaks the box and returns a stable pointer.
    pub fn from_box(boxed: Box<T>) -> Self {
        // SAFETY: Box::into_raw always returns a non-null pointer.
        Self(unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) })
    }

    /// Drops the pointee by reconstructing the [`Box`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other references to the data exist and
    /// that this method is called at most once per pointer.
    pub unsafe fn free_unchecked(self) {
        // SAFETY: The caller guarantees exclusive access and single-free.
        unsafe {
            drop(Box::from_raw(self.0.as_ptr()));
        }
    }

    /// Returns a shared reference to the pointee with an explicit lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer has not been freed and that the
    /// returned reference does not outlive the actual allocation.
    pub unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY: The caller guarantees the pointer is still valid.
        unsafe { self.0.as_ref() }
    }
}

// SAFETY:
//
// Leaked pointer acts as a shared reference when sent to other threads. The
// allocation is guaranteed to be alive until explicitly freed during the
// maintenance phase (single-threaded). T must be `Sync` because the pointer
// exposes a shared reference to T.
unsafe impl<T: Sync> Send for LeakedBoxPtr<T> {}

// SAFETY:
//
// Leaked pointer only exposes immutable access to pointee type. Sharing it
// gives concurrent read-only access, which is safe when pointee is `Sync`.
unsafe impl<T: Sync> Sync for LeakedBoxPtr<T> {}

impl<T> Copy for LeakedBoxPtr<T> {}

impl<T> Clone for LeakedBoxPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
