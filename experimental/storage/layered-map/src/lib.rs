// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use bitvec::prelude::*;
pub use layer::MapLayer;
pub use map::LayeredMap;
use std::hash::Hash;

mod dropper;
mod flatten_perfect_tree;
pub mod iterator;
mod layer;
mod map;
mod metrics;
mod node;
pub(crate) mod r#ref;
#[cfg(test)]
mod tests;
mod utils;

/// When recursively creating a new `MapLayer` (a crit bit tree overlay), partitioning and passing
/// down `Vec<(K, Option<V>)>` would mean a lot of memory allocation. That's why we require
/// `Key: Clone` and clone the key and value only when the leaf node is created.
pub trait Key: Clone + Hash + Eq + Ord {}

impl<T: Clone + Hash + Eq + Ord> Key for T {}

/// Similar to `Key`, we require `Value: Clone`, another reason being it's tricky to figure out the
/// lifetime if `get()` returns a reference to the value -- we simply clone the value.
pub trait Value: Clone {}

impl<T: Clone> Value for T {}

/// Wrapper for the hash value of a key.
///
/// Addressing bits in the order of MSB to LSB, so that sorting by the numeric order is the same
/// as sorting by bits.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct KeyHash(u64);

impl KeyHash {
    fn bit(&self, n: usize) -> bool {
        *self
            .0
            .view_bits::<Msb0>()
            .get(n)
            .expect("Caller guarantees range.")
    }

    fn iter_bits(&self) -> impl Iterator<Item = bool> + '_ {
        self.0.view_bits::<Msb0>().iter().by_vals()
    }
}
