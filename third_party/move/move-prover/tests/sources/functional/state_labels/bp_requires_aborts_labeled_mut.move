// separate_baseline: prophecy
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `requires_of` and `aborts_of` are single-state predicates, but their `&mut`
// arguments are still `Old(...)`-wrapped before lowering. Under a labeled
// state, the wrap must resolve via a per-label witness; otherwise it falls
// into the unsupported-`Old` generic path.

module 0x42::bp_requires_aborts_labeled_mut {
    struct Counter has copy, drop, store { value: u64 }

    fun checked_inc(c: &mut Counter): u64 {
        assert!(c.value < 100, 1);
        c.value = c.value + 1;
        c.value
    }
    spec checked_inc {
        requires c.value <= 100;
        aborts_if c.value >= 100;
        ensures c.value == old(c).value + 1;
        ensures result == c.value;
    }

    fun inc_twice_with_bp(c: &mut Counter) {
        checked_inc(c);
        checked_inc(c);
    }
    spec inc_twice_with_bp {
        requires c.value <= 98;
        ensures exists S in *:
            (S |~ requires_of<checked_inc>(c)) &&
            (S |~ !aborts_of<checked_inc>(c));
    }
}
