// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An inline function with a specification of its own is verified against
// it, as the Move Prover verifies it, while its calls are expanded: a caller
// proves what the body does, not what the specification says.
module 0x42::inline_spec_false {
    inline fun bad_inc(x: u64): u64 {
        x + 2
    }
    spec bad_inc {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1; // error: the body adds 2
    }

    fun call(x: u64): u64 {
        bad_inc(x)
    }
    spec call {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 2;
    }
}
