// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The value of a specification function outside its domain is unspecified.
// The Move Prover defines it there by the body.
module 0x42::spec_fun_domain_false {
    spec fun successor(x: u8): num { x + 1 }

    fun outside(x: u8): u8 { x }
    spec outside {
        ensures successor(300) == 301; // error: 300 is outside the domain
    }

    spec fun wrapping_successor(x: u64): u64 { int2bv(x + 1) }

    fun wraps(x: u64): u64 { x }
    spec wraps {
        requires x == MAX_U64;
        ensures wrapping_successor(x) == x + 1; // error: wraps to zero
    }
}
