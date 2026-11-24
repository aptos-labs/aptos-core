// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The derived contract of a function without a specification that writes a
// field of a freshly packed enum value. The field sits at a different
// position in the packed variant than in the first variant carrying it.
module 0x42::pack_hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

// Both fields have the same type.
module 0x42::pack_same_type {
    use 0x42::pack_hof::apply_to;
    enum S has copy, drop, store { A { v: u64, w: u64 }, B { w: u64, v: u64 }, C { z: u64 } }

    public fun set_b(x: &mut S) { *x = S::B { w: 10, v: 20 }; x.v = 99 }
    public fun set_b_local(x: &mut S) { let b = S::B { w: 10, v: 20 }; b.v = 99; *x = b }
    public fun set_b_twice(x: &mut S) { *x = S::B { w: 10, v: 20 }; x.v = 99; x.w = 7; x.v = 5 }
    public fun set_a(x: &mut S) { *x = S::A { v: 1, w: 2 }; x.w = 7 }
    public fun set_c(x: &mut S) { *x = S::C { z: 1 }; x.v = 99 }

    fun c_set_b(s: &mut S) { apply_to(s, set_b) }
    spec c_set_b { aborts_if false; ensures s == S::B { w: 10, v: 99 }; }
    fun c_set_b_incorrect(s: &mut S) { apply_to(s, set_b) }
    spec c_set_b_incorrect { ensures s == S::B { w: 99, v: 20 }; }

    fun c_set_b_local(s: &mut S) { apply_to(s, set_b_local) }
    spec c_set_b_local { aborts_if false; ensures s == S::B { w: 10, v: 99 }; }
    fun c_set_b_local_incorrect(s: &mut S) { apply_to(s, set_b_local) }
    spec c_set_b_local_incorrect { ensures s == S::B { w: 99, v: 20 }; }

    fun c_set_b_twice(s: &mut S) { apply_to(s, set_b_twice) }
    spec c_set_b_twice { aborts_if false; ensures s == S::B { w: 7, v: 5 }; }
    fun c_set_b_twice_incorrect(s: &mut S) { apply_to(s, set_b_twice) }
    spec c_set_b_twice_incorrect { ensures s == S::B { w: 5, v: 7 }; }

    fun c_set_a(s: &mut S) { apply_to(s, set_a) }
    spec c_set_a { aborts_if false; ensures s == S::A { v: 1, w: 7 }; }
    fun c_set_a_incorrect(s: &mut S) { apply_to(s, set_a) }
    spec c_set_a_incorrect { ensures s == S::A { v: 7, w: 2 }; }

    // The packed variant has no field `v`: the write aborts.
    fun c_set_c(s: &mut S) { apply_to(s, set_c) }
    spec c_set_c { aborts_if true; }
    fun c_set_c_incorrect(s: &mut S) { apply_to(s, set_c) }
    spec c_set_c_incorrect { aborts_if false; }
}

// The field at the first variant's position has another type in the packed
// variant.
module 0x42::pack_other_type {
    use 0x42::pack_hof::apply_to;
    enum S has copy, drop, store { A { v: u64, w: u64 }, B { w: bool, v: u64 } }

    public fun set_b(x: &mut S) { *x = S::B { w: true, v: 20 }; x.v = 99 }

    fun c_set_b(s: &mut S) { apply_to(s, set_b) }
    spec c_set_b { aborts_if false; ensures s == S::B { w: true, v: 99 }; }
    fun c_set_b_incorrect(s: &mut S) { apply_to(s, set_b) }
    spec c_set_b_incorrect { ensures s == S::B { w: true, v: 20 }; }
}

// Control: the same update written in a specification is translated as written.
module 0x42::pack_written {
    enum S has copy, drop, store { A { v: u64, w: u64 }, B { w: u64, v: u64 } }

    fun mk(): S { S::B { w: 10, v: 99 } }
    spec mk { ensures result == update_field(S::B { w: 10, v: 20 }, v, 99); }
    fun mk_incorrect(): S { S::B { w: 99, v: 20 } }
    spec mk_incorrect { ensures result == update_field(S::B { w: 10, v: 20 }, v, 99); }
}

// Lean leaves the true spec of `apply_to` unproved.
// exclude_for: lean
