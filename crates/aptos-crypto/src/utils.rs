// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Utility functions for general-purpose operations.
//! Currently contains only a `powers()` function for computing sequential powers of a base element.

use ark_std::ops::MulAssign;
use num_traits::One;

/// Returns the first `count` powers of a given `base` element, so
/// [1, base, base^2, base^3, ..., base^{count - 1}]
pub fn powers<T>(base: T, count: usize) -> Vec<T>
where
    T: MulAssign + One + Copy,
{
    let mut powers = Vec::with_capacity(count);
    let mut current = T::one();

    for _ in 0..count {
        powers.push(current);
        current *= base;
    }

    powers
}
