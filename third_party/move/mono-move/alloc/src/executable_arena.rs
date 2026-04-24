// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use bumpalo::Bump;
use std::{fmt, ptr::NonNull};

/// A pointer into an executable's private arena. The pointer is valid for the
/// lifetime of the executable (arena is dropped together with the executable).
///
/// # Safety model
///
/// Dereferencing is **unsafe** - the caller must ensure the executable that
/// owns the arena allocation has not been dropped. When arena is dropped, it
/// must be guaranteed there are no outstanding pointers.
#[repr(transparent)]
pub struct ExecutableArenaPtr<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> ExecutableArenaPtr<T> {
    /// Returns a shared reference to the pointee with the specified lifetime.
    ///
    /// # Safety
    ///
    ///   1. The caller must ensure the executable's arena that owns the
    ///      allocation is alive and has not been reset or dropped.
    ///   2. The lifetime is the lifetime of the executable arena.
    pub unsafe fn as_ref_unchecked<'arena>(&self) -> &'arena T {
        // SAFETY: The caller ensures the arena is still alive / not dropped.
        unsafe { self.0.as_ref() }
    }

    /// Returns a mutable reference to the pointee with the specified lifetime.
    ///
    /// # Safety
    ///
    ///   1. The caller must ensure the executable's arena that owns the
    ///      allocation is alive and has not been reset or dropped.
    ///   2. The caller must ensure exclusive access to the pointee.
    ///   3. The lifetime is the lifetime of the executable arena.
    pub unsafe fn as_mut_unchecked<'arena>(&mut self) -> &'arena mut T {
        // SAFETY: The caller ensures the arena is still alive / not dropped,
        // and that no other references to the pointee exist.
        unsafe { self.0.as_mut() }
    }

    /// Returns the underlying `NonNull` pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.0
    }
}

// This type can be duplicated using bitwise copy.
impl<T: ?Sized> Copy for ExecutableArenaPtr<T> {}

impl<T: ?Sized> Clone for ExecutableArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// TODO: Only needed because MicroOp derives Debug. Remove once MicroOp
// uses a manual Debug impl or no longer stores ExecutableArenaPtr fields.
impl<T: ?Sized> fmt::Debug for ExecutableArenaPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExecutableArenaPtr({:p})", self.0)
    }
}

// SAFETY: This pointer acts as a shared reference when sent to other threads.
// It is allocated in the executable's arena, and therefore is guaranteed to
// be alive while the executable is alive. Because executables are never
// dropped during concurrent execution (INVARIANT), there is no need to require
// T to be `Send`. However, T has to be `Sync` because the pointer does expose
// a shared reference to T.
unsafe impl<T: Sync + ?Sized> Send for ExecutableArenaPtr<T> {}

// SAFETY: This pointer is `Sync` because it provides read-only access to T
// when shared between threads, which is safe if T is also `Sync`.
unsafe impl<T: Sync + ?Sized> Sync for ExecutableArenaPtr<T> {}

/// A bump arena for per-executable allocations that cannot be shared across
/// different executable versions, i.e., can be invalidated by upgrades. For
/// example, functions can change on upgrades and so their code is allocated
/// per executable.
///
/// # Safety model
///
/// On allocation, returns [`ExecutableArenaPtr`] pointers. Dereferencing this
/// pointer is safe if the arena and all its allocations are alive.
///
/// It is the user's responsibility to ensure that no outstanding pointers exist
/// when the arena is dropped. In practice, the executable owns the arena and
/// upholds this invariant by guaranteeing that all arena pointers are contained
/// within the executable itself, and they are dropped before the arena.
pub struct ExecutableArena {
    bump: Bump,
}

impl ExecutableArena {
    /// Creates a new, empty executable arena.
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    /// Allocates a value in the arena, returning a pointer to it.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the value fails.
    pub fn alloc<T>(&self, value: T) -> ExecutableArenaPtr<T> {
        ExecutableArenaPtr(NonNull::from(self.bump.alloc(value)))
    }

    /// Copies a slice into the arena, returning a pointer to it.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the slice fails.
    pub fn alloc_slice_copy<T: Copy>(&self, values: &[T]) -> ExecutableArenaPtr<[T]> {
        ExecutableArenaPtr(NonNull::from(self.bump.alloc_slice_copy(values)))
    }

    /// Allocates a slice in the arena from an exact-size iterator.
    ///
    /// ## Panics
    ///
    /// Panics if reserving space for the slice fails.
    pub fn alloc_slice_fill_iter<T, I>(&self, iter: I) -> ExecutableArenaPtr<[T]>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        ExecutableArenaPtr(NonNull::from(self.bump.alloc_slice_fill_iter(iter)))
    }
}

impl Default for ExecutableArena {
    fn default() -> Self {
        Self::new()
    }
}
