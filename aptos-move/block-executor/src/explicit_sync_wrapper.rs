// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic,
};

// Parallel algorithms often guarantee a sequential use of certain
// data structures, or parts of the data-structures (like elements of
// a vector).  The rust compiler can not prove the safety of even
// slightly complex parallel algorithms.

/// ExplicitSyncWrapper is meant to be used in parallel algorithms
/// where we can prove that there will be no concurrent access to the
/// underlying object (or its elements).  Use with caution - only when
/// the safety can be proven.
#[derive(Debug)]
pub struct ExplicitSyncWrapper<T> {
    value: UnsafeCell<T>,
}

pub struct Guard<'a, T> {
    lock: &'a ExplicitSyncWrapper<T>,
}

impl<T> ExplicitSyncWrapper<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn acquire(&self) -> Guard<'_, T> {
        atomic::fence(atomic::Ordering::Acquire);
        Guard { lock: self }
    }

    pub(crate) fn unlock(&self) {
        atomic::fence(atomic::Ordering::Release);
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    pub fn dereference(&self) -> &T {
        unsafe { &*self.value.get() }
    }

    // This performs the acquire fence so temporal reasoning on the result
    // of the dereference is valid, and then returns a reference with the
    // same lifetime as the wrapper (unlike acquire which returns a guard).
    pub fn fence_and_dereference(&self) -> &T {
        atomic::fence(atomic::Ordering::Acquire);
        self.dereference()
    }

    pub fn dereference_mut<'a>(&self) -> &'a mut T {
        unsafe { &mut *self.value.get() }
    }
}

impl<T> Guard<'_, T> {
    pub fn dereference(&self) -> &T {
        self.lock.dereference()
    }

    pub fn dereference_mut(&mut self) -> &mut T {
        self.lock.dereference_mut()
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.lock.dereference()
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.lock.dereference_mut()
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

unsafe impl<T> Sync for ExplicitSyncWrapper<T> {}
