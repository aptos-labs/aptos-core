// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The quotient and remainder of a variable divisor, related by a built-in
/// lemma.
module 0x42::proof_div_mod {
    fun position(i: u64, s: u64): (u64, u64) {
        (i / s, i % s)
    }
    spec position {
        requires s > 0;
        ensures result_1 * s + result_2 == i;
        ensures result_2 < s;
    }
}
