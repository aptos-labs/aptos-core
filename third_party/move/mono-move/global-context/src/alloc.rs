// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Allocation primitives used by the execution context.
//!
//! Three allocation strategies are used:
//!
//!
//! # Leaked box
//!
//! [`LeakedBoxPtr<T>`] is individually heap-allocated via `Box::leak`. Have
//! to be freed manually. Used for [`crate::Executable`] entries in the cache
//! that need explicit non-bulk deallocation.
//!
//! # Executable arena
//!
//! [`ExecutableArena`] bump-allocates data, returning
//! [`ExecutableArenaPtr<T>`].
//! Arena lives until the [`crate::Executable`] is dropped or reset. When
//! dropped or reset, users must guarantee no live pointers to arena exist.
//! Used for data that does not need explicit memory management and goes
//! away together with the executable.
//!
//! # Global arena
//!
//! [`GlobalArena`] bump-allocates data, returning [`GlobalArenaPtr<T>`]. Data
//! in this arena lives indefinitely until reset (or until it is dropped) due
//! to memory constraints on the system. Used for data that out-lives
//! executables and is not subject to code upgrade invalidation, e.g.,
//! identifiers such as function and struct names, or types.

use crate::context::{ExecutionContextScope, MaintenanceContextScope};
use bumpalo::Bump;
use crossbeam::utils::CachePadded;
use parking_lot::{Mutex, MutexGuard};
use std::{
    hash::{Hash, Hasher},
    ptr::NonNull,
};

/// A stable non-arena pointer allocated by leaking the [`Box`]. Always stays
/// alive unless it has been freed.
///
/// # Safety
///
/// After freeing the data behind the pointer, dereferencing is UB.
#[repr(transparent)]
pub(crate) struct LeakedBoxPtr<T>(NonNull<T>);

impl<T> LeakedBoxPtr<T> {
    /// Creates a new pointer from [`Box`]-ed hip allocation.
    pub(crate) fn from_box(data: Box<T>) -> Self {
        Self(NonNull::from(Box::leak(data)))
    }

    /// Frees the allocation behind the pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no references to this data exist.
    pub(crate) unsafe fn free_unchecked(self) {
        // SAFETY:
        //   Caller must ensure safety preconditions.
        unsafe {
            drop(Box::from_raw(self.0.as_ptr()));
        }
    }

    /// Returns a reference with the specified lifetime to the pointed-to value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the box must not have been freed since this
    /// pointer was created.
    pub(crate) unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY:
        //   Caller must ensure safety preconditions.
        unsafe { self.0.as_ref() }
    }
}

// SAFETY:
//   This is a pointer to Box::leak-ed heap memory. Sending it to another
//   thread is safe if T: Send, because the allocation is stable and no thread
//   holds a mutable reference after the leak.
unsafe impl<T: Send> Send for LeakedBoxPtr<T> {}

// SAFETY:
//   Sharing the pointer across threads is safe if T: Sync, because the
//   pointed-to value is immutable after the box is leaked.
unsafe impl<T: Sync> Sync for LeakedBoxPtr<T> {}

impl<T> Copy for LeakedBoxPtr<T> {}

impl<T> Clone for LeakedBoxPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A stable pointer allocated in [`ExecutableArena`]. Guaranteed to be always
/// alive unless the arena has been reset or dropped.
///
/// # Safety
///
/// After resetting or dropping the arena, dereferencing this pointer is UB.
#[repr(transparent)]
pub(crate) struct ExecutableArenaPtr<T>(NonNull<T>);

impl<T> ExecutableArenaPtr<T> {
    /// Returns a reference with the specified lifetime to the pointed-to value.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// 1. The [`ExecutableArena`] must not have been dropped or reset since
    ///    this pointer was created.
    /// 2. The returned reference must not outlive the owning
    ///    [`crate::Executable`].
    pub(crate) unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY:
        //   Caller must ensure safety preconditions.
        unsafe { self.0.as_ref() }
    }
}

// SAFETY:
//   This is a pointer to bump-allocated data with a stable address in the
//   arena. Sending it to another thread is safe if T: Send; the allocation
//   is immutable after it is placed in the arena as long as the arena is
//   not dropped or reset. Neither flush nor reset happens concurrently.
unsafe impl<T: Send> Send for ExecutableArenaPtr<T> {}

// SAFETY:
//   Sharing the pointer across threads is safe if T: Sync; the pointed-to value
//   is immutable after allocation and the arena and its address is stable. Note
//   that arena is not dropped or reset concurrently.
unsafe impl<T: Sync> Sync for ExecutableArenaPtr<T> {}

impl<T> Copy for ExecutableArenaPtr<T> {}

impl<T> Clone for ExecutableArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Bump arena for a single [`crate::Executable`] instance.
pub(crate) struct ExecutableArena {
    arena: Bump,
}

impl ExecutableArena {
    /// Creates a new arena for [`crate::Executable`] instance.
    pub(crate) fn new() -> Self {
        Self { arena: Bump::new() }
    }

    /// Allocates data in the arena and returns a pointer to it.
    ///
    /// # Safety
    ///
    /// Arena must not be dropped or reset if there exists an alive pointer to
    /// its data.
    pub(crate) unsafe fn alloc<T>(&self, data: T) -> ExecutableArenaPtr<T> {
        ExecutableArenaPtr(NonNull::from(self.arena.alloc(data)))
    }
}

impl Default for ExecutableArena {
    fn default() -> Self {
        Self::new()
    }
}

/// A stable pointer allocated in [`GlobalArena`]. Guaranteed to be always alive
/// unless the arena has been dropped or reset.
///
/// # Safety
///
/// After dropping or resetting the arena, dereferencing this pointer is UB.
#[repr(transparent)]
pub struct GlobalArenaPtr<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> GlobalArenaPtr<T> {
    /// Creates a [`GlobalArenaPtr`] that points to statically allocated data.
    ///
    /// Unlike arena-allocated pointers, the result is never invalidated by
    /// arena reset or arena drop: the static data lives for the entire lifetime
    /// of a program.
    pub(crate) fn from_static(data: &'static T) -> Self {
        #[allow(clippy::incompatible_msrv)]
        GlobalArenaPtr(NonNull::from_ref(data))
    }

    /// Unsafely casts this arena pointer to a reference with the
    /// specified lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// 1. The arena has not been reset or dropped since this pointer
    ///    was created.
    /// 2. The returned reference does not outlive the arena's actual lifetime.
    pub(crate) unsafe fn as_ref_unchecked<'a>(&self) -> &'a T {
        // SAFETY:
        //   Caller must ensure safety preconditions.
        unsafe { self.0.as_ref() }
    }

    /// Returns a reference to the pointed-to value, valid for the execution
    /// context scope's lifetime `'ctx`.
    ///
    /// Safe because [`ExecutionContextScope`] proves an `ExecutionContext`
    /// guard is held, which prevents arena flush for `'ctx`.
    pub(crate) fn as_ref<'ctx>(&self, _scope: &ExecutionContextScope<'ctx>) -> &'ctx T {
        // SAFETY: ExecutionContextScope proves the arena is stable for 'ctx.
        unsafe { self.as_ref_unchecked() }
    }
}

// SAFETY:
//   This is just a pointer to arena-allocated data. If T is Send, then
//   sending the pointer to another thread is safe because the arena keeps
//   the allocation stable until reset or drop. Reset and drop never happen
//   concurrently.
unsafe impl<T: ?Sized + Send> Send for GlobalArenaPtr<T> {}

// SAFETY:
//   This is just a pointer to arena-allocated data. If T is Sync, then sharing
//   the pointer across threads is safe because the arena keeps the allocation
//   stable and T can be safely accessed from multiple threads. There is also
//   no concurrent deallocation.
unsafe impl<T: ?Sized + Sync> Sync for GlobalArenaPtr<T> {}

impl<T: ?Sized> Copy for GlobalArenaPtr<T> {}

impl<T: ?Sized> Clone for GlobalArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implement pointer-based equality so we can use pointers as keys.
impl<T: ?Sized> PartialEq for GlobalArenaPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl<T: ?Sized> Eq for GlobalArenaPtr<T> {}

// Implement pointer-based hash so we can use pointers as keys.
impl<T: ?Sized> Hash for GlobalArenaPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.as_ptr() as *const T).hash(state)
    }
}

/// Arena allocator API that never releases memory.
pub trait GlobalArena {
    /// Allocates a value in the arena, returning a stable pointer to it.
    fn alloc<T>(&self, value: T) -> GlobalArenaPtr<T>;

    /// Allocates a string in the arena, returning a stable pointer to it.
    fn alloc_str(&self, s: &str) -> GlobalArenaPtr<str>;

    /// Allocates a slice by copying from the source slice, returning a
    /// stable pointer.
    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> GlobalArenaPtr<[T]>;

    /// Returns the number of allocated bytes in the arena (may include
    /// padding, etc.)
    fn allocated_bytes(&self) -> usize;
}

/// A pool of bump allocators that never deallocate individual items.
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
    pub(crate) fn with_capacity(arena_capacity: usize) -> Self {
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
    pub fn lock_arena(&self, idx: usize) -> Option<ArenaGuard<'_>> {
        assert!(idx < self.num_arenas);
        Some(ArenaGuard {
            guard: self.arenas[idx].try_lock()?,
        })
    }

    /// Locks the arena at a specific index, blocking until the lock is
    /// acquired.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn lock_arena_blocking(&self, idx: usize) -> ArenaGuard<'_> {
        assert!(idx < self.num_arenas);
        ArenaGuard {
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
    pub(crate) fn allocated_bytes(&self, idx: usize) -> usize {
        assert!(idx < self.num_arenas);
        self.arenas[idx].lock().allocated_bytes()
    }

    /// Returns the total number of allocated bytes across all arenas in
    /// the pool.
    pub(crate) fn allocated_bytes_sum(&self) -> usize {
        self.arenas
            .iter()
            .map(|arena| arena.lock().allocated_bytes())
            .sum()
    }

    /// Resets all arenas in the pool, making all allocations invalid.
    ///
    /// # Safety
    ///
    /// The caller must ensure all interners and the executable cache have been
    /// cleared — no live `GlobalArenaPtr` values may point into these arenas.
    /// [`MaintenanceContextScope`] proves the write lock is held (no concurrent
    /// execution contexts); `unsafe` captures the semantic invariant that the
    /// type system cannot verify: all caches must be cleared before this call.
    pub(crate) unsafe fn reset(&self, _scope: &MaintenanceContextScope<'_>) {
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

impl GlobalArena for ArenaGuard<'_> {
    fn alloc<T>(&self, value: T) -> GlobalArenaPtr<T> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc(value)))
    }

    fn alloc_str(&self, s: &str) -> GlobalArenaPtr<str> {
        GlobalArenaPtr(NonNull::from(self.guard.alloc_str(s)))
    }

    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> GlobalArenaPtr<[T]> {
        let slice_ref = self.guard.alloc_slice_copy(src);
        GlobalArenaPtr(NonNull::from(slice_ref))
    }

    fn allocated_bytes(&self) -> usize {
        self.guard.allocated_bytes()
    }
}
