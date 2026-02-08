// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Stable pointer wrapper that properly implements Send + Sync.

use std::ptr::NonNull;

/// A stable pointer that is Send + Sync when T is Send + Sync.
///
/// This wraps `NonNull<T>` and provides safe `Send + Sync` implementations
/// for pointers that are guaranteed to remain valid and stable (never
/// invalidated or moved) for the lifetime of the data structure.
///
/// # Safety
///
/// The pointer must:
/// - Remain valid and never be invalidated (except via explicit flush operations)
/// - Never be moved in memory (stable address)
/// - Be properly synchronized through the owning data structure
#[repr(transparent)]
#[derive(Debug)]
pub struct StablePtr<T>(NonNull<T>);

impl<T> StablePtr<T> {
    /// Creates a new stable pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The pointer remains valid for the lifetime of the interner
    /// - The pointer is never invalidated (except via flush)
    /// - The pointer's target is never moved in memory
    /// - Access to the pointer's target is properly synchronized
    pub unsafe fn new(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }

    /// Returns the underlying `NonNull<T>`.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.0
    }

    /// Returns a raw pointer to the value.
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }

    /// Returns a mutable raw pointer to the value.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    /// Dereferences the pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization when accessing the value.
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> Clone for StablePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StablePtr<T> {}

impl<T> PartialEq for StablePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for StablePtr<T> {}

// SAFETY: StablePtr is Send if T is Send, because:
// - The underlying data is owned by the arena
// - The arena properly synchronizes access through locks/atomics
// - Pointers are stable and never invalidated (except via explicit flush)
unsafe impl<T: Send> Send for StablePtr<T> {}

// SAFETY: StablePtr is Sync if T is Sync, because:
// - The underlying data is owned by the arena
// - The arena properly synchronizes access through locks/atomics
// - Multiple threads can safely hold copies of the pointer
unsafe impl<T: Sync> Sync for StablePtr<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_ptr_basic() {
        let value = 42;
        let ptr = NonNull::from(&value);
        let stable = unsafe { StablePtr::new(ptr) };

        unsafe {
            assert_eq!(*stable.as_ref(), 42);
        }
    }

    #[test]
    fn test_stable_ptr_clone() {
        let value = 42;
        let ptr = NonNull::from(&value);
        let stable1 = unsafe { StablePtr::new(ptr) };
        let stable2 = stable1.clone();

        assert_eq!(stable1, stable2);
    }

    #[test]
    fn test_stable_ptr_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<StablePtr<String>>();
        assert_sync::<StablePtr<String>>();
    }
}
