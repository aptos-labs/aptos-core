// separate_baseline: path
// Copyright © Aptos Foundation
// Data invariants of a subtree committed to a global resource are asserted on
// the *definite pack base*: the highest reference every borrow chain of the
// dying leaf passes through. This covers a field borrowed conditionally from
// either of two resources (base = the field reference) as well as sequential
// field writes through one resource reference, where the compiler's temp reuse
// makes the chains re-converge at the resource reference (base = that
// reference).
module 0x42::data_inv_cond_field_global {
    struct Inner has store { x: u64 }
    spec Inner {
        invariant x > 0;
    }

    struct R1 has key { i: Inner }

    struct R2 has key { i: Inner }

    fun violates(a: address, c: bool) acquires R1, R2 {
        let r = if (c) &mut R1[a].i else &mut R2[a].i;
        r.x = 0; // error: data invariant does not hold
    }

    fun preserves(a: address, c: bool) acquires R1, R2 {
        let r = if (c) &mut R1[a].i else &mut R2[a].i;
        r.x = 7;
    }

    struct C has key { a: u64, b: u64 }
    spec C {
        invariant a <= b;
    }

    fun seq_violates(x: u64, y: u64) acquires C {
        let c = &mut C[@0x42];
        c.a = x;
        c.b = y; // error: data invariant does not hold
    }

    fun seq_preserves(x: u64) acquires C {
        let c = &mut C[@0x42];
        c.a = x;
        c.b = x + 1;
    }
}
