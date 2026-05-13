// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Tests that `old()` is rejected in non-post proof contexts.
module 0x42::proof_old_rejected {

    struct Counter has drop {
        value: u64,
    }

    // old() in entry-point assert should be rejected.
    fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment {
        requires c.value < MAX_U64;
        ensures c.value == old(c.value) + 1;
    } proof {
        assert old(c.value) + 1 <= MAX_U64; // error: old() not allowed
    }
}
