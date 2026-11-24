// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A sequence of derived writes to fields of a wide enum variant: each write
// updates one field of the variant.
module 0x42::hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

module 0x42::many {
    use 0x42::hof::apply_to;
    enum S has copy, drop, store {
        V { f1: u64, f2: u64, f3: u64, f4: u64, f5: u64, f6: u64, f7: u64, f8: u64, f9: u64, f10: u64, f11: u64, f12: u64 }
    }
    enum M has copy, drop, store {
        A { a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, p: u64 },
        B { a6: u64, a5: u64, a4: u64, a3: u64, a2: u64, a1: u64 }
    }

    public fun set_many(x: &mut S) { x.f1 = 1; x.f2 = 2; x.f1 = 3; x.f3 = 4 }
    public fun set_shared(x: &mut M) { x.a1 = 1; x.a2 = 2; x.a3 = 3 }

    fun c_many(s: &mut S) { apply_to(s, set_many) }
    spec c_many {
        aborts_if false;
        ensures s.f1 == 3 && s.f2 == 2 && s.f3 == 4;
        ensures s.f4 == old(s.f4) && s.f5 == old(s.f5) && s.f6 == old(s.f6) && s.f7 == old(s.f7);
        ensures s.f8 == old(s.f8) && s.f9 == old(s.f9) && s.f10 == old(s.f10) && s.f11 == old(s.f11);
        ensures s.f12 == old(s.f12);
    }
    fun c_many_incorrect(s: &mut S) { apply_to(s, set_many) }
    spec c_many_incorrect { ensures s.f1 == 1; }

    fun c_shared(s: &mut M) { apply_to(s, set_shared) }
    spec c_shared {
        aborts_if false;
        ensures (old(s) is A) == (s is A);
        ensures s.a1 == 1 && s.a2 == 2 && s.a3 == 3;
        ensures s.a4 == old(s.a4) && s.a5 == old(s.a5) && s.a6 == old(s.a6);
        ensures (old(s) is A) ==> s.p == old(s.p);
    }
    fun c_shared_incorrect(s: &mut M) { apply_to(s, set_shared) }
    spec c_shared_incorrect { ensures s is A; }
}

// A long sequence of derived writes below a variant field: reading the field
// between two writes does not repeat the value written to.
module 0x42::nested {
    use 0x42::hof::apply_to;
    enum I has copy, drop, store { V1 { n: u64, m: u64 }, V2 { n: u64 } }
    enum O has copy, drop, store { A { t: I, k: u64 }, B { k: u64 } }

    public fun set_nested(x: &mut O) {
        x.t.n = 1; x.t.m = 2; x.t.n = 3; x.t.m = 4; x.t.n = 5;
        x.t.m = 6; x.t.n = 7; x.t.m = 8; x.t.n = 9; x.t.m = 10;
        x.k = 5;
        x.t.n = 11; x.t.m = 12; x.t.n = 13; x.t.m = 14; x.t.n = 15;
        x.t.m = 16; x.t.n = 17; x.t.m = 18; x.t.n = 19; x.t.m = 20;
    }

    fun c_nested(s: &mut O) { apply_to(s, set_nested) }
    spec c_nested {
        aborts_if !(s is A);
        aborts_if (s is A) && !(s.t is V1);
        ensures s.t.n == 19 && s.t.m == 20 && s.k == 5;
        ensures (s is A) && (s.t is V1);
    }
    fun c_nested_incorrect(s: &mut O) { apply_to(s, set_nested) }
    spec c_nested_incorrect { ensures s.t.n == 1; }
    fun c_nested_aborts_incorrect(s: &mut O) { apply_to(s, set_nested) }
    spec c_nested_aborts_incorrect { aborts_if !(s is A); }
}

// exclude_for: lean
// The Lean backend does not verify the higher-order function `apply_to` itself.
