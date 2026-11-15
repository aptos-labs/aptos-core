// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::inherited_strictness_false {
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
        // Without strictness the callee may abort.
        aborts_if x == 0 with 1; // error: the callee may abort
    }
}
