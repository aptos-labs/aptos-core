// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cell::UnsafeCell;

// Parallel algorithms often guarantee a sequential use of certain
// data structures, or parts of the data-structures (like elements of
// a vector).  The rust compiler can not profe the safety of even
// slightly complex parallel algorithms.

/// ExplicitSyncWrapper is meant to be used in parallel algorithms
/// where we can prove that there will be no concurrent access to the
/// underlying object (or its elements).  Use with caution - only when
/// the safety can be proven.
pub struct ExplicitSyncWrapper<T> {
    value: UnsafeCell<T>,
}

impl<T> ExplicitSyncWrapper<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn get_mut<'a>(&self) -> &'a mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn get<'a>(&self) -> &'a T {
        unsafe { &mut *self.value.get() }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

unsafe impl<T> Sync for ExplicitSyncWrapper<T> {}
