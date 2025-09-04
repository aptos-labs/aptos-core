// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::{
    fmt::Debug,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

/// A generic [`IdGenerator`] trait, it's intentionally generic to allow for different
/// orders and types of `Id`
pub trait IdGenerator<Id: Copy + Debug> {
    /// Retrieves a new `Id`
    fn next(&self) -> Id;
}

/// A generic in order [`IdGenerator`] using an [`AtomicU32`] to guarantee uniqueness
#[derive(Debug)]
pub struct U32IdGenerator {
    inner: AtomicU32,
}

impl U32IdGenerator {
    /// Creates a new [`U32IdGenerator`] initialized to `0`
    pub const fn new() -> Self {
        Self::new_with_value(0)
    }

    /// Creates a new [`U32IdGenerator`] with an `initial_value`
    pub const fn new_with_value(initial_value: u32) -> Self {
        Self {
            inner: AtomicU32::new(initial_value),
        }
    }
}

impl IdGenerator<u32> for U32IdGenerator {
    /// Retrieves the next ID, wrapping on overflow
    #[inline]
    fn next(&self) -> u32 {
        self.inner.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for U32IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// A generic in order [`IdGenerator`] using an [`AtomicU64`] to guarantee uniqueness
#[derive(Debug, Default)]
pub struct U64IdGenerator {
    inner: AtomicU64,
}

impl U64IdGenerator {
    /// Creates a new [`U64IdGenerator`] initialized to `0`
    pub const fn new() -> Self {
        Self::new_with_value(0)
    }

    /// Creates a new [`U64IdGenerator`] with an `initial_value`
    pub const fn new_with_value(initial_value: u64) -> Self {
        Self {
            inner: AtomicU64::new(initial_value),
        }
    }
}
impl IdGenerator<u64> for U64IdGenerator {
    /// Retrieves the next ID, wrapping on overflow
    #[inline]
    fn next(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_generation() {
        let id_generator = U64IdGenerator::new();

        for i in 0..10 {
            assert_eq!(i, id_generator.next())
        }
    }

    #[test]
    fn check_overflow() {
        let id_generator = U64IdGenerator::new_with_value(u64::MAX);
        assert_eq!(u64::MAX, id_generator.next());
        assert_eq!(0, id_generator.next());
    }
}
