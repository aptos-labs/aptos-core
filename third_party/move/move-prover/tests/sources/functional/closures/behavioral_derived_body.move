// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Behavioral predicates over a function without a specification are
// interpreted by its body when the body describes its results, `&mut`
// post-states, and aborts exactly.
module 0x42::behavioral_derived_body {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun double(x: u64): u64 {
        x * 2
    }

    fun inc_b(s: &mut S) {
        s.b = s.b + 1;
    }

    fun succ_double(x: u64): u64 {
        double(x) + 1
    }
    spec succ_double(x: u64): u64 {
        pragma opaque;
        ensures result == result_of<double>(x) + 1;
        aborts_if aborts_of<double>(x);
        aborts_if !aborts_of<double>(x) && result_of<double>(x) == MAX_U64;
    }

    fun succ_double_wrong(x: u64): u64 {
        double(x) + 1
    }
    spec succ_double_wrong(x: u64): u64 {
        pragma opaque;
        ensures result == result_of<double>(x) + 2; // error: the body adds one
        aborts_if aborts_of<double>(x);
        aborts_if !aborts_of<double>(x) && result_of<double>(x) == MAX_U64;
    }

    fun bump(s: &mut S) {
        inc_b(s);
    }
    spec bump(s: &mut S) {
        pragma opaque;
        ensures ensures_of<inc_b>(old(s), s);
        aborts_if aborts_of<inc_b>(s);
    }

    fun bump_wrong(s: &mut S) {
        inc_b(s);
    }
    spec bump_wrong(s: &mut S) {
        pragma opaque;
        ensures ensures_of<inc_b>(old(s), update_field(s, a, 7)); // error: `a` is unchanged
        aborts_if aborts_of<inc_b>(s);
    }
}
