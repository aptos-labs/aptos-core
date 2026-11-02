// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A caller owes a callee's precondition at the call, as the Move Prover
// asserts it there: whether the callee's body is inlined or its contract
// used, and whether the caller states a specification or not.
module 0x42::callee_precondition_false {
    fun half(x: u64): u64 {
        x / 2
    }
    spec half {
        requires x % 2 == 0;
        ensures result * 2 == x;
    }

    fun opaque_half(x: u64): u64 {
        x / 2
    }
    spec opaque_half {
        pragma opaque;
        requires x % 2 == 0;
        ensures result * 2 == x;
    }

    // Without a specification, a function is verified for the
    // preconditions of its calls.
    fun even_half(): u64 {
        half(4)
    }

    fun odd_half(): u64 {
        half(3) // error: 3 is odd
    }

    fun guarded(x: u64): u64 {
        if (x % 2 == 0) half(x) else 0
    }
    spec guarded {
        ensures result <= x;
    }

    fun unguarded(x: u64): u64 {
        half(x) // error: x may be odd
    }
    spec unguarded {
        ensures result <= x;
    }

    fun unguarded_opaque(x: u64): u64 {
        opaque_half(x) // error: x may be odd
    }
    spec unguarded_opaque {
        ensures result <= x;
    }
}
