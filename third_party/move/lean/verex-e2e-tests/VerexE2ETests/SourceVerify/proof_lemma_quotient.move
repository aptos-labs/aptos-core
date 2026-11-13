// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A `lemma` item's own proof reads its conclusion in the normal form: the
/// quotient of a dividend its `requires` bounds is `/`, as a leaf's.
module 0x42::proof_lemma_quotient {
    fun scaled(x: u64, c: u64): u64 {
        x / c * c
    }
    spec scaled {
        requires c > 0;
        ensures result <= x;
    }
}
