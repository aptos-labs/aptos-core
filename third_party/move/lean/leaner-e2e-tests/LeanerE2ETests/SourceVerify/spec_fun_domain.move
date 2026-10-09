// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A specification function is defined where its parameters fit their declared
// types. An `int2bv` the compiler types at `num` wraps at `u64`.
module 0x42::spec_fun_domain {
    spec fun successor(x: u8): num { x + 1 }

    fun in_domain(x: u8): u64 { (x as u64) + 1 }
    spec in_domain {
        ensures result == successor(x);
    }

    // A `num` parameter has no bound.
    spec fun successor_num(x: num): num { x + 1 }

    fun unbounded(x: u8): u8 { x }
    spec unbounded {
        ensures successor_num(300) == 301;
    }

    spec fun wrapping_successor(x: u64): u64 { int2bv(x + 1) }

    fun wraps(x: u64): u64 { x }
    spec wraps {
        requires x == MAX_U64;
        ensures wrapping_successor(x) == 0;
    }
}
