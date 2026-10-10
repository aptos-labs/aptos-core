// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// At a call to an opaque function, `ensures_of` and `aborts_of` of that
// function relate the `&mut` argument's value before the call to its value
// after. A caller's postcondition which misstates the post-state must fail.
module 0x42::behavioral_mut_pre_state {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun inc_b(s: &mut S) {
        s.b = s.b + 1;
    }
    spec inc_b(s: &mut S) {
        pragma opaque;
        ensures s == update_field(old(s), b, old(s).b + 1);
        aborts_if s.b == MAX_U64;
    }

    fun only_call(s: &mut S) {
        inc_b(s);
    }
    spec only_call(s: &mut S) {
        pragma opaque;
        ensures ensures_of<inc_b>(old(s), s);
        aborts_if aborts_of<inc_b>(s);
    }

    fun only_call_wrong(s: &mut S) {
        inc_b(s);
    }
    spec only_call_wrong(s: &mut S) {
        pragma opaque;
        ensures ensures_of<inc_b>(old(s), update_field(s, a, 7)); // error: the post-state keeps `a`
        aborts_if aborts_of<inc_b>(s);
    }

    fun call_then_write(s: &mut S) {
        inc_b(s);
        s.a = 1;
    }
    spec call_then_write(s: &mut S) {
        pragma opaque;
        ensures exists w: S: ensures_of<inc_b>(old(s), w) && s == update_field(w, a, 1);
        aborts_if aborts_of<inc_b>(s);
    }

    fun call_then_write_wrong(s: &mut S) {
        inc_b(s);
        s.a = 1;
    }
    spec call_then_write_wrong(s: &mut S) {
        pragma opaque;
        ensures ensures_of<inc_b>(old(s), s); // error: the write to `a` follows the call
        aborts_if aborts_of<inc_b>(s);
    }
}
