// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `data invariant k of S` routes a function constructing two struct types: the first clause of
/// each owes a different witness.
module 0x42::proof_invariant_of {
    struct Even has copy, drop {
        value: u64,
    }
    spec Even {
        invariant exists half: u64: value == 2 * half;
    }

    struct Triple has copy, drop {
        value: u64,
    }
    spec Triple {
        invariant exists third: u64: value == 3 * third;
    }

    fun make(half: u64, third: u64): (Even, Triple) {
        (Even { value: half + half }, Triple { value: third + third + third })
    }
    spec make {
        requires half + half <= MAX_U64;
        requires third + third + third <= MAX_U64;
    }
}
