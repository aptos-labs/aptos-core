// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A module's `pragma aborts_if_is_strict` holds for each of its functions: a
// contract without `aborts_if` states that its function does not abort.
module 0x42::inherited_strictness {
    spec module {
        pragma aborts_if_is_strict;
    }

    fun callee(x: u64): u64 { x }
    spec callee {
        pragma opaque;
        ensures result == x;
    }

    fun caller(x: u64): u64 {
        if (callee(x) == 0) abort 1;
        x
    }
    spec caller {
        aborts_if x == 0 with 1;
    }
}
