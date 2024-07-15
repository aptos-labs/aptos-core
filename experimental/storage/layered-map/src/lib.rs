// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use bitvec::{order::Msb0, view::BitView};
pub use layer::MapLayer;
pub use map::LayeredMap;

mod dropper;
mod layer;
mod map;
mod metrics;
mod node;
pub(crate) mod r#ref;

#[cfg(test)]
mod tests;

/// When recursively creating a new `MapLayer` (a crit bit tree overlay), partitioning and passing
/// down `Vec<(K, Option<V>)>` would mean a lot of memory allocation. That's why we require
/// `Key: Clone` and clone the key and value only when the leaf node is created.
pub trait Key: Clone + Eq {
    fn iter_bits(&self) -> impl Iterator<Item = bool>;

    fn bit(&self, depth: usize) -> bool;
}

impl Key for HashValue {
    fn iter_bits(&self) -> impl Iterator<Item = bool> {
        self.iter_bits()
    }

    fn bit(&self, depth: usize) -> bool {
        *self.as_slice().view_bits::<Msb0>().get(depth).unwrap()
    }
}

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
