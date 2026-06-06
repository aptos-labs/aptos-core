// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Spec functions with `&mut` parameters and `old(p)` called under labeled
// state ranges (`..S |~`, `S.. |~`). Covers single-`&mut`, multi-`&mut` of
// the same type, and multi-`&mut` of mixed types.

module 0x42::param_old_labeled_repro {
    struct Counter has copy, drop, store { value: u64 }
    struct Stamp   has copy, drop, store { ticks: u64 }

    // --- 1. Single `&mut`. -----------------------------------------------------

    spec fun counter_increased(c: &mut Counter): bool {
        old(c).value < c.value
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
            (..S |~ counter_increased(c)) &&
            (S.. |~ counter_increased(c));
    }

    // --- 2. Multi-`&mut`, same underlying type. --------------------------------

    spec fun both_increased(a: &mut Counter, b: &mut Counter): bool {
        old(a).value < a.value && old(b).value < b.value
    }

    fun inc_both(a: &mut Counter, b: &mut Counter) {
        a.value = a.value + 1;
        b.value = b.value + 1;
    }
    spec inc_both {
        ensures exists S in *:
            (..S |~ both_increased(a, b)) &&
            (S.. |~ both_increased(a, b));
    }

    // --- 3. Multi-`&mut`, mixed underlying types. ------------------------------

    spec fun pair_progressed(c: &mut Counter, s: &mut Stamp): bool {
        old(c).value < c.value && old(s).ticks < s.ticks
    }

    fun step_both(c: &mut Counter, s: &mut Stamp) {
        c.value = c.value + 1;
        s.ticks = s.ticks + 1;
    }
    spec step_both {
        ensures exists S in *:
            (..S |~ pair_progressed(c, s)) &&
            (S.. |~ pair_progressed(c, s));
    }
}
