// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use bitvec::prelude::*;

/// When recursively creating a new `MapLayer` (a crit bit tree overlay), passing down `Vec<(K, Option<V>)>`
/// That's why we require `Key: Clone` and clone the key and value only when the leaf node is
/// created.
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
