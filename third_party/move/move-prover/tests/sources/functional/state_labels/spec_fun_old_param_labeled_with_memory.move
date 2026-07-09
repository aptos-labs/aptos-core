// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Labeled spec-function call where the callee uses `old(p)` on a `&mut`
// parameter and also reads global memory.

module 0x42::param_old_labeled_mixed {
    struct Counter has copy, drop, store { value: u64 }
    struct Cap has key { max: u64 }

    spec fun under_cap(c: &mut Counter, addr: address): bool {
        old(c).value < c.value && c.value <= global<Cap>(addr).max
    }

    fun inc_under_cap_twice(c: &mut Counter, addr: address) acquires Cap {
        if (c.value + 1 < Cap[addr].max) {
            c.value = c.value + 1;
        };
        if (c.value + 1 < Cap[addr].max) {
            c.value = c.value + 1;
        };
    }
    spec inc_under_cap_twice {
        requires c.value + 2 < Cap[addr].max;
        ensures exists S in *:
            (..S |~ under_cap(c, addr)) &&
            (S.. |~ under_cap(c, addr));
    }
}
