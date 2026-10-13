// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A specification function over a `&mut` parameter reads its value before
// (`old(c)`) and after; a call passes the argument's value before and after.
module 0x42::spec_mut_params {
    struct Counter has copy, drop, store { value: u64 }

    spec fun increased(c: &mut Counter): bool {
        old(c).value < c.value
    }

    fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment {
        ensures increased(c);
    }

    fun keep(_c: &mut Counter) {}
    spec keep {
        ensures increased(_c); // the counter is unchanged
    }
}
