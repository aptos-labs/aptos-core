// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::ptr::NonNull;

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
