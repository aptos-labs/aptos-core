// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A spec function with a `&mut` parameter but no `old()` is *not* called with
// doubled `(Old(p), p)` args, yet under a labeled state range its `&mut` slot
// must still evaluate at the labeled state rather than the ambient state.

module 0x42::pure_spec_fun_labeled_mut {
    struct Counter has copy, drop, store { value: u64 }

    spec fun small(c: &mut Counter): bool {
        c.value < 100
    }

    fun inc(c: &mut Counter) {
        c.value = c.value + 1;
    }

    fun inc_twice(c: &mut Counter) {
        inc(c);
        inc(c)
    }
    spec inc_twice {
        ensures exists S in *:
            (..S |~ small(c)) && (S.. |~ small(c));
    }
}
