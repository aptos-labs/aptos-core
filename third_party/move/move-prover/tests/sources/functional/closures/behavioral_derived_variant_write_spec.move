// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A field update of an enum value which comes from the specification of a
// called function, not from a write through a borrow of the field: it says
// nothing about a receiver in a variant without the field, so the derivation
// must not treat it like a write.
module 0x42::hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

module 0x42::spec_src {
    use 0x42::hof::apply_to;
    enum I has copy, drop, store { V1 { n: u64 } }
    enum O has copy, drop, store { A { t: I, k: u64 }, B { k: u64 } }

    fun g(o: &mut O) { if (o is A) o.t = I::V1 { n: 7 } }
    spec g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures o == update_field(old(o), t, I::V1 { n: 7 });
    }

    public fun h(x: &mut O) { g(x); let n = x.t.n; x.k = n; }

    fun c_true(s: &mut O) { apply_to(s, h) }
    spec c_true {
        pragma aborts_if_is_partial;
        ensures (old(s) is A) ==> s.k == 7;
    }
    // For a `B`, the specification of `g` leaves the value unspecified, so
    // this abort condition does not follow.
    fun c_aborts_incorrect(s: &mut O) { apply_to(s, h) }
    spec c_aborts_incorrect { aborts_if !(s is A); }
}

// exclude_for: lean
// The Lean backend does not verify the higher-order function `apply_to` itself.
