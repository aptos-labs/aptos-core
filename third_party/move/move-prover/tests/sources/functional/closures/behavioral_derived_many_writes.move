// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Chains of field updates on a struct with many fields. Each update names
// the receiver once per other field, so a chain of ten updates must not be
// expanded into one term.
module 0x42::many_writes_hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

// A function without a specification, interpreted by its body when used
// as a function value.
module 0x42::many_writes_derived {
    use 0x42::many_writes_hof::apply_to;

    struct S has copy, drop, store {
        f1: u64, f2: u64, f3: u64, f4: u64, f5: u64, f6: u64,
        f7: u64, f8: u64, f9: u64, f10: u64, f11: u64, f12: u64,
    }

    public fun set_many(x: &mut S) {
        x.f1 = 1; x.f2 = 2; x.f3 = 3; x.f1 = 4; x.f4 = 5;
        x.f5 = 6; x.f2 = 7; x.f6 = 8; x.f7 = 9; x.f8 = 10
    }

    fun call_set_many(s: &mut S) { apply_to(s, set_many) }
    spec call_set_many {
        aborts_if false;
        ensures s.f1 == 4 && s.f2 == 7 && s.f3 == 3 && s.f4 == 5;
        ensures s.f5 == 6 && s.f6 == 8 && s.f7 == 9 && s.f8 == 10;
        ensures s.f9 == old(s.f9) && s.f10 == old(s.f10);
        ensures s.f11 == old(s.f11) && s.f12 == old(s.f12);
    }

    fun call_set_many_incorrect(s: &mut S) { apply_to(s, set_many) }
    spec call_set_many_incorrect {
        ensures s.f1 == 1; // error: `f1` is written again with 4
    }
}

// A written specification stating the same ten writes as a chain of
// `update_field`.
module 0x42::many_writes_spec {
    struct S has copy, drop, store {
        f1: u64, f2: u64, f3: u64, f4: u64, f5: u64, f6: u64,
        f7: u64, f8: u64, f9: u64, f10: u64, f11: u64, f12: u64,
    }

    fun set_many(x: &mut S) {
        x.f1 = 1; x.f2 = 2; x.f3 = 3; x.f1 = 4; x.f4 = 5;
        x.f5 = 6; x.f2 = 7; x.f6 = 8; x.f7 = 9; x.f8 = 10
    }
    spec set_many {
        aborts_if false;
        ensures x == update_field(update_field(update_field(update_field(update_field(
            update_field(update_field(update_field(update_field(update_field(
                old(x), f1, 1), f2, 2), f3, 3), f1, 4), f4, 5),
            f5, 6), f2, 7), f6, 8), f7, 9), f8, 10);
    }

    fun set_many_incorrect(x: &mut S) {
        x.f1 = 1; x.f2 = 2; x.f3 = 3; x.f1 = 4; x.f4 = 5;
        x.f5 = 6; x.f2 = 7; x.f6 = 8; x.f7 = 9; x.f8 = 10
    }
    spec set_many_incorrect {
        ensures x == update_field(update_field(update_field(update_field(update_field(
            update_field(update_field(update_field(update_field(update_field(
                old(x), f1, 1), f2, 2), f3, 3), f1, 4), f4, 5),
            f5, 6), f2, 7), f6, 8), f7, 9), f9, 10); // error: the last write is to `f8`
    }
}

// Lean leaves the true specs of `apply_to` and `call_set_many` unproved.
// exclude_for: lean
