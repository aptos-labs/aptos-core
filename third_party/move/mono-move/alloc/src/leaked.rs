// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::{
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

/// A stable pointer obtained by leaking a [`Box`].
///
/// Used for data that must remain at a fixed address but requires non-bulk
/// deallocation. For example, some executables stored in the cache can be
/// dropped  if there are newer (upgraded) versions, but other cache entries
/// must remain alive.
///
/// # Safety model
///
/// The pointer is created by leaking a [`Box`], ensuring a stable address on
/// the heap. The data is freed via [`LeakedBoxPtr::free_unchecked`]. Freeing
/// is **unsafe** - the caller must guarantee that no other references to the
/// data exist.
#[repr(transparent)]
pub struct LeakedBoxPtr<T>(NonNull<T>);

impl<T> LeakedBoxPtr<T> {
    /// Leaks the box and returns a stable pointer.
    pub fn from_box(boxed: Box<T>) -> Self {
        // SAFETY: Box::into_raw always returns a non-null pointer.
        Self(unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) })
    }

    /// Frees allocated data.
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

// SAFETY: This pointer acts as a shared reference when sent to other threads.
// The allocation is guaranteed to be alive until explicitly freed. When freed,
// exclusive access is guaranteed (see `free_unchecked` safety requirement). T
// must be `Sync` because the pointer exposes a shared reference to T.
unsafe impl<T: Sync> Send for LeakedBoxPtr<T> {}

// SAFETY: This pointer only exposes immutable access to T. Sharing the pointer
// provides concurrent read-only access, which is safe when T is `Sync`.
unsafe impl<T: Sync> Sync for LeakedBoxPtr<T> {}

// Can be duplicated with bitwise copy.
impl<T> Copy for LeakedBoxPtr<T> {}

impl<T> Clone for LeakedBoxPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Versioned slot holding [`LeakedBoxPtr<T>`]s.
pub struct VersionedLeakedBoxPtr<T> {
    base: AtomicPtr<T>,
    // TODO:
    //   In the future, other versions will be place here.
    //   Right now, only one base (storage) version exists.
}

impl<T> VersionedLeakedBoxPtr<T> {
    /// Creates an empty slot.
    pub fn new() -> Self {
        Self {
            base: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Returns the current pointer if set, or [`None`] otherwise.
    pub fn load(&self) -> Option<LeakedBoxPtr<T>> {
        // TODO:
        //   In the future, the algorithm will be more involved: we need to
        //   find the right version to return based on the version specified
        //   by the user.
        let raw = self.base.load(Ordering::Acquire);
        NonNull::new(raw).map(LeakedBoxPtr)
    }

    /// Sets the slot if it was empty before.
    ///
    /// On race, if slot is already occupied, returns the input pointer back
    /// in `Err` so the caller can either free it or adopt the winning version
    /// via [`VersionedLeakedBoxPtr::load`].
    pub fn init(&self, ptr: LeakedBoxPtr<T>) -> Result<(), LeakedBoxPtr<T>> {
        let raw = ptr.0.as_ptr();
        // On success: Release publishes the pointee's initialization to
        // subsequent `load` readers (Acquire).
        // On failure: the caller only observes that some other initialization
        // happened; any subsequent `load` performs its own Acquire, so using
        // Relaxed is sufficient for the failure ordering.
        match self
            .base
            .compare_exchange(ptr::null_mut(), raw, Ordering::Release, Ordering::Relaxed)
        {
            Ok(_) => Ok(()),
            Err(_) => Err(ptr),
        }
    }

    /// Atomically swaps null in and returns the previous content if any.
    pub fn clear(&self) -> Option<LeakedBoxPtr<T>> {
        // TODO: Revisit GC storey with Zaptos.
        let raw = self.base.swap(ptr::null_mut(), Ordering::AcqRel);
        NonNull::new(raw).map(LeakedBoxPtr)
    }
}

impl<T> Default for VersionedLeakedBoxPtr<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leak<T>(value: T) -> LeakedBoxPtr<T> {
        LeakedBoxPtr::from_box(Box::new(value))
    }

    #[test]
    fn empty_loads_none() {
        let slot: VersionedLeakedBoxPtr<u64> = VersionedLeakedBoxPtr::new();
        assert!(slot.load().is_none());
    }

    #[test]
    fn init_then_load() {
        let slot = VersionedLeakedBoxPtr::new();
        let p = leak(42u64);
        assert!(slot.init(p).is_ok());
        let observed = slot.load().expect("load returns Some");
        // SAFETY: slot still holds a valid leaked box.
        unsafe { assert_eq!(*observed.as_ref_unchecked(), 42) };
        // SAFETY: drain the slot and free the allocation to avoid leaks.
        unsafe { slot.clear().unwrap().free_unchecked() };
    }

    #[test]
    fn second_init_loses_race() {
        let slot = VersionedLeakedBoxPtr::new();
        let winner = leak(1u64);
        let loser = leak(2u64);
        assert!(slot.init(winner).is_ok());
        let back = match slot.init(loser) {
            Ok(()) => panic!("second init should fail"),
            Err(back) => back,
        };
        // The caller either frees the returned loser or adopts the winner.
        // SAFETY: `back` is the loser, exclusive to this test.
        unsafe { back.free_unchecked() };
        // SAFETY: drain the winner.
        unsafe { slot.clear().unwrap().free_unchecked() };
    }

    #[test]
    fn clear_returns_previous_then_empty() {
        let slot = VersionedLeakedBoxPtr::new();
        let p = leak(7u64);
        assert!(slot.init(p).is_ok());
        let out = slot.clear().expect("returns previous");
        // SAFETY: `out` is the leaked box we installed above, still alive.
        unsafe { assert_eq!(*out.as_ref_unchecked(), 7) };
        // SAFETY: free the box to avoid leaks in tests.
        unsafe { out.free_unchecked() };
        assert!(slot.load().is_none());
    }

    #[test]
    fn clear_on_empty_is_none() {
        let slot: VersionedLeakedBoxPtr<u64> = VersionedLeakedBoxPtr::new();
        assert!(slot.clear().is_none());
    }
}
