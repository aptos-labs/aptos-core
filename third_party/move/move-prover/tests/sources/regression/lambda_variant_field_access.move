// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The derived specification of a lambda must read and write enum fields in every variant
// the access may apply to.

module 0x42::lambda_variant_field_read {
    enum E has copy, drop { A { x: u64 }, B { x: u64, y: u64 } }

    fun apply(f: |E|u64 has copy+drop, e: E): u64 {
        f(e)
    }

    /// Must verify: `x` exists in both variants.
    public fun read_shared(e: E): u64 {
        let f = |v: E| v.x;
        apply(f, e)
    }
    spec read_shared {
        ensures result == e.x;
    }

    /// Must fail: `B { x: 1, .. }` and `B { x: 2, .. }` give different results.
    public fun two(e1: E, e2: E): bool {
        let f = |v: E| v.x;
        apply(f, e1) == apply(f, e2)
    }
    spec two {
        requires e1 is B;
        requires e2 is B;
        ensures result;
    }
}

module 0x42::lambda_variant_field_write {
    // `rr` and `ss` share offset 1. `a_touch_rr` mentions `rr` first.
    enum E has copy, drop { A { p: u64, rr: u64 }, B { q: u64, ss: u64 } }

    enum F has copy, drop { A { x: u64 }, B { x: u64, y: u64 } }

    fun a_touch_rr(e: &E): u64 {
        e.rr
    }

    fun apply_e(f: |&mut E| has drop, e: &mut E) {
        f(e)
    }
    spec apply_e {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(e);
        ensures ensures_of<f>(old(e), e);
    }

    fun apply_f(g: |&mut F| has drop, f: &mut F) {
        g(f)
    }
    spec apply_f {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<g>(f);
        ensures ensures_of<g>(old(f), f);
    }

    /// Must verify: the lambda writes `ss`.
    public fun write_ss(e: &mut E) {
        apply_e(|s| { s.ss = 7; }, e)
    }
    spec write_ss {
        requires e is B;
        aborts_if false;
        ensures e.ss == 7;
    }

    /// Must fail: `ss` may have changed.
    public fun write_ss_unchanged(e: &mut E) {
        apply_e(|s| { s.ss = 7; }, e)
    }
    spec write_ss_unchanged {
        requires e is B;
        aborts_if false;
        ensures e == old(e);
    }

    /// Must verify: `x` exists in both variants.
    public fun write_shared(f: &mut F) {
        apply_f(|s| { s.x = 5; }, f)
    }
    spec write_shared {
        aborts_if false;
        ensures f.x == 5;
    }
}
