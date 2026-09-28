// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A function type's Boogie name records where its parameters end and its results begin:
// `|u64, u8| u8` and `|u64| (u8, u8)` are distinct. Both are reachable from one verification
// target, since function types are emitted per shard.

module 0x42::fun_type_arity_injectivity {

    fun two_in(_x: u64, y: u8): u8 { y }

    fun two_out(_x: u64): (u8, u8) { (1, 2) }

    /// Must verify. Reaches both arity splits of `u64, u8, u8`.
    public fun both_arities(): bool {
        let a: |u64, u8|u8 has copy + drop = two_in;
        let b: |u64|(u8, u8) has copy + drop = two_out;
        let r = a(1, 7);
        let (p, q) = b(3);
        r == 7 && p == 1 && q == 2
    }

    spec both_arities {
        ensures result == true;
    }
}
