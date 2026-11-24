// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The derived contract of a function without a specification models writes
// through enum variant fields.
module 0x42::hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

// Single variant.
module 0x42::single {
    use 0x42::hof::apply_to;
    enum S has copy, drop, store { V1 { v: u64, w: u64 } }
    public fun reset(x: &mut S) { x.v = 0 }
    public fun set(x: &mut S, n: u64) { x.v = n }
    public fun bump(x: &mut S) { x.v = x.v + 1 }
    public fun two(x: &mut S) { x.v = 1; x.w = 2; x.v = 3 }
    public fun via_ref(x: &mut S) { let r = &mut x.v; *r = 5 }

    fun c_reset(s: &mut S) { apply_to(s, reset) }
    spec c_reset { aborts_if false; ensures s.v == 0; ensures s.w == old(s.w); ensures s == S::V1 { v: 0, w: old(s.w) }; }
    fun c_reset_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_incorrect { ensures s.v == old(s.v); }
    fun c_reset_w_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_w_incorrect { ensures s.w == 0; }

    fun c_set(s: &mut S, n: u64) { apply_to(s, |x| set(x, n)) }
    spec c_set { aborts_if false; ensures s.v == n; ensures s.w == old(s.w); }
    fun c_set_incorrect(s: &mut S, n: u64) { apply_to(s, |x| set(x, n)) }
    spec c_set_incorrect { ensures s.v == old(s.v); }

    fun c_bump(s: &mut S) { apply_to(s, bump) }
    spec c_bump { aborts_if s.v == MAX_U64; ensures s.v == old(s.v) + 1; ensures s.w == old(s.w); }
    fun c_bump_incorrect(s: &mut S) { apply_to(s, bump) }
    spec c_bump_incorrect { aborts_if false; ensures s.v == old(s.v); }

    fun c_two(s: &mut S) { apply_to(s, two) }
    spec c_two { aborts_if false; ensures s.v == 3 && s.w == 2; }
    fun c_two_incorrect(s: &mut S) { apply_to(s, two) }
    spec c_two_incorrect { ensures s.v == 1; }

    fun c_via_ref(s: &mut S) { apply_to(s, via_ref) }
    spec c_via_ref { aborts_if false; ensures s.v == 5; ensures s.w == old(s.w); }
    fun c_via_ref_incorrect(s: &mut S) { apply_to(s, via_ref) }
    spec c_via_ref_incorrect { ensures s.v == old(s.v); }
}

// Several variants; the field is in all or some of them, at different offsets.
module 0x42::multi {
    use 0x42::hof::apply_to;
    enum S has copy, drop, store { A { v: u64, w: u64 }, B { w: u64, v: u64 }, C { z: u64 } }
    enum T has copy, drop, store { A { v: u64, w: u64 }, B { v: u64 } }
    public fun reset(x: &mut S) { x.v = 0 }
    public fun reset_t(x: &mut T) { x.v = 0 }
    public fun reset_z(x: &mut S) { x.z = 0 }

    fun c_reset(s: &mut S) { apply_to(s, reset) }
    spec c_reset {
        aborts_if s is C;
        ensures s.v == 0;
        ensures (old(s) is A) ==> (s is A) && s.w == old(s.w);
        ensures (old(s) is B) ==> (s is B) && s.w == old(s.w);
    }
    fun c_reset_abort_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_abort_incorrect { aborts_if false; }
    fun c_reset_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_incorrect { ensures s.v == old(s.v); }
    fun c_reset_variant_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_variant_incorrect { ensures s is A; }
    fun c_reset_w_incorrect(s: &mut S) { apply_to(s, reset) }
    spec c_reset_w_incorrect { ensures s.w == 0; }

    fun c_reset_t(s: &mut T) { apply_to(s, reset_t) }
    spec c_reset_t {
        aborts_if false;
        ensures s.v == 0;
        ensures (old(s) is A) == (s is A);
        ensures (old(s) is A) ==> s.w == old(s.w);
    }
    fun c_reset_t_incorrect(s: &mut T) { apply_to(s, reset_t) }
    spec c_reset_t_incorrect { ensures s is B; }

    fun c_reset_z(s: &mut S) { apply_to(s, reset_z) }
    spec c_reset_z { aborts_if !(s is C); ensures s == S::C { z: 0 }; }
    fun c_reset_z_incorrect(s: &mut S) { apply_to(s, reset_z) }
    spec c_reset_z_incorrect { aborts_if false; }
}

// Nesting: struct in enum, enum in struct, enum in enum, generic.
module 0x42::nested {
    use 0x42::hof::apply_to;
    struct P has copy, drop, store { a: u64, b: u64 }
    enum E has copy, drop, store { V1 { p: P, n: u64 } }
    struct W has copy, drop, store { e: E, k: u64 }
    enum O has copy, drop, store { X { e: E, k: u64 }, Y { k: u64 } }
    enum G<T> has copy, drop, store { V1 { t: T, n: u64 } }

    public fun in_enum(x: &mut E) { x.p.a = 7 }
    public fun in_struct(x: &mut W) { x.e.n = 7 }
    public fun in_enum_enum(x: &mut O) { x.e.p.b = 7 }
    public fun gen<T: drop>(x: &mut G<T>, t: T) { x.t = t }
    public fun gen_n<T>(x: &mut G<T>) { x.n = 9 }

    fun c_in_enum(s: &mut E) { apply_to(s, in_enum) }
    spec c_in_enum { aborts_if false; ensures s.p.a == 7 && s.p.b == old(s.p.b) && s.n == old(s.n); }
    fun c_in_enum_incorrect(s: &mut E) { apply_to(s, in_enum) }
    spec c_in_enum_incorrect { ensures s == old(s); }

    fun c_in_struct(s: &mut W) { apply_to(s, in_struct) }
    spec c_in_struct { aborts_if false; ensures s.e.n == 7 && s.e.p == old(s.e.p) && s.k == old(s.k); }
    fun c_in_struct_incorrect(s: &mut W) { apply_to(s, in_struct) }
    spec c_in_struct_incorrect { ensures s == old(s); }

    fun c_in_enum_enum(s: &mut O) { apply_to(s, in_enum_enum) }
    spec c_in_enum_enum {
        aborts_if s is Y;
        ensures s.e.p.b == 7 && s.e.p.a == old(s.e.p.a) && s.e.n == old(s.e.n) && s.k == old(s.k);
    }
    fun c_in_enum_enum_incorrect(s: &mut O) { apply_to(s, in_enum_enum) }
    spec c_in_enum_enum_incorrect { ensures s == old(s); }

    fun c_gen(s: &mut G<u8>) { apply_to(s, |x| gen(x, 3)) }
    spec c_gen { aborts_if false; ensures s.t == 3 && s.n == old(s.n); }
    fun c_gen_incorrect(s: &mut G<u8>) { apply_to(s, |x| gen(x, 3)) }
    spec c_gen_incorrect { ensures s.t == old(s.t); }
    fun c_gen_n<T>(s: &mut G<T>) { apply_to(s, gen_n) }
    spec c_gen_n { aborts_if false; ensures s.n == 9 && s.t == old(s.t); }
    fun c_gen_n_incorrect<T>(s: &mut G<T>) { apply_to(s, gen_n) }
    spec c_gen_n_incorrect { ensures s.n == old(s.n); }
}

// A vector element in a variant field.
module 0x42::vec {
    use 0x42::hof::apply_to;
    enum L has copy, drop, store { V1 { items: vector<u64>, n: u64 } }
    public fun elem(x: &mut L) { x.items[0] = 4 }

    fun c_elem(s: &mut L) { apply_to(s, elem) }
    spec c_elem { aborts_if len(s.items) == 0; ensures s.items[0] == 4 && s.n == old(s.n) && len(s.items) == len(old(s.items)); }
    fun c_elem_incorrect(s: &mut L) { apply_to(s, elem) }
    spec c_elem_incorrect { ensures s == old(s); }
}

// The enum and its setter are in another module than the user.
module 0x42::other {
    use 0x42::hof::apply_to;
    use 0x42::single::{Self, S};
    fun c_reset(s: &mut S) { apply_to(s, single::reset) }
    spec c_reset { aborts_if false; ensures s.v == 0; ensures s.w == old(s.w); }
    fun c_reset_incorrect(s: &mut S) { apply_to(s, single::reset) }
    spec c_reset_incorrect { ensures s.v == old(s.v); }
}

// Positional variant fields, carried by some variants only.
module 0x42::pos {
    use 0x42::hof::apply_to;
    enum P has copy, drop, store { A(u64, bool), B(u64), C }
    public fun set0(x: &mut P) { x.0 = 5 }
    public fun set1(x: &mut P) { x.1 = true }

    fun c_set0(s: &mut P) { apply_to(s, set0) }
    spec c_set0 {
        aborts_if s is C;
        ensures (old(s) is A) ==> s == P::A(5, old(s).1);
        ensures (old(s) is B) ==> s == P::B(5);
    }
    fun c_set0_incorrect(s: &mut P) { apply_to(s, set0) }
    spec c_set0_incorrect { ensures s == old(s); }
    fun c_set0_abort_incorrect(s: &mut P) { apply_to(s, set0) }
    spec c_set0_abort_incorrect { aborts_if false; }

    fun c_set1(s: &mut P) { apply_to(s, set1) }
    spec c_set1 { aborts_if !(s is A); ensures s == P::A(old(s).0, true); }
    fun c_set1_incorrect(s: &mut P) { apply_to(s, set1) }
    spec c_set1_incorrect { ensures s.0 == 0; }
}

// The base changes variant before or after a field write.
module 0x42::shift {
    use 0x42::hof::apply_to;
    enum S has copy, drop, store { A { v: u64, w: u64 }, B { v: u64 }, C { z: u64 } }

    public fun whole_then_field(x: &mut S) { *x = S::B { v: 1 }; x.v = 2 }
    fun c_wtf(s: &mut S) { apply_to(s, whole_then_field) }
    spec c_wtf { aborts_if false; ensures s == S::B { v: 2 }; }
    fun c_wtf_incorrect(s: &mut S) { apply_to(s, whole_then_field) }
    spec c_wtf_incorrect { ensures s == S::B { v: 1 }; }

    public fun field_then_whole(x: &mut S) { x.v = 2; *x = S::C { z: 3 } }
    fun c_ftw(s: &mut S) { apply_to(s, field_then_whole) }
    spec c_ftw { aborts_if s is C; ensures s == S::C { z: 3 }; }
    fun c_ftw_incorrect(s: &mut S) { apply_to(s, field_then_whole) }
    spec c_ftw_incorrect { aborts_if false; }

    public fun to_c_then_field(x: &mut S) { *x = S::C { z: 1 }; x.v = 2 }
    fun c_tctf(s: &mut S) { apply_to(s, to_c_then_field) }
    spec c_tctf { aborts_if true; }
    fun c_tctf_incorrect(s: &mut S) { apply_to(s, to_c_then_field) }
    spec c_tctf_incorrect { aborts_if false; ensures s == S::C { z: 1 }; }

    public fun read_after_change(x: &mut S) { x.v = 1; *x = S::C { z: 0 }; let a = x.v; x.z = a }
    fun c_rac(s: &mut S) { apply_to(s, read_after_change) }
    spec c_rac { aborts_if true; }
    fun c_rac_incorrect(s: &mut S) { apply_to(s, read_after_change) }
    spec c_rac_incorrect { aborts_if s is C; }

    public fun to_a_then_w(x: &mut S) { *x = S::A { v: 0, w: 0 }; x.w = 4 }
    fun c_tatw(s: &mut S) { apply_to(s, to_a_then_w) }
    spec c_tatw { aborts_if false; ensures s == S::A { v: 0, w: 4 }; }
    fun c_tatw_incorrect(s: &mut S) { apply_to(s, to_a_then_w) }
    spec c_tatw_incorrect { ensures s == S::A { v: 0, w: 0 }; }

    // The second borrow is of a field only `A` carries.
    public fun two_borrows(x: &mut S) { let r1 = &mut x.v; *r1 = 1; let r2 = &mut x.w; *r2 = 2 }
    fun c_tb(s: &mut S) { apply_to(s, two_borrows) }
    spec c_tb { aborts_if !(s is A); ensures s == S::A { v: 1, w: 2 }; }
    fun c_tb_incorrect(s: &mut S) { apply_to(s, two_borrows) }
    spec c_tb_incorrect { aborts_if s is C; }
    fun c_tb_v_incorrect(s: &mut S) { apply_to(s, two_borrows) }
    spec c_tb_v_incorrect { ensures s.v == old(s.v); }

    public fun swap(x: &mut S) { let t = x.v; x.v = x.w; x.w = t }
    fun c_swap(s: &mut S) { apply_to(s, swap) }
    spec c_swap { aborts_if !(s is A); ensures s.v == old(s.w) && s.w == old(s.v); }
    fun c_swap_incorrect(s: &mut S) { apply_to(s, swap) }
    spec c_swap_incorrect { ensures s == old(s); }

    public fun imm_then_write(x: &mut S) { let r = &x.v; let a = *r; x.v = a + 1; x.w = a }
    fun c_itw(s: &mut S) { apply_to(s, imm_then_write) }
    spec c_itw { aborts_if !(s is A); aborts_if s.v == MAX_U64; ensures s.v == old(s.v) + 1 && s.w == old(s.v); }
    fun c_itw_incorrect(s: &mut S) { apply_to(s, imm_then_write) }
    spec c_itw_incorrect { ensures s.w == s.v; }

    public fun let_then_write(x: &mut S) { let a = x.v; x.v = 7; x.w = a }
    fun c_ltw(s: &mut S) { apply_to(s, let_then_write) }
    spec c_ltw { aborts_if !(s is A); ensures s == S::A { v: 7, w: old(s.v) }; }
    fun c_ltw_incorrect(s: &mut S) { apply_to(s, let_then_write) }
    spec c_ltw_incorrect { ensures s.w == 7; }
}

// Phantom and nested generic enums.
module 0x42::gens {
    use 0x42::hof::apply_to;
    enum Ph<phantom T> has copy, drop, store { A { v: u64 }, B { u: u64 } }
    enum G<T> has copy, drop, store { V1 { t: T, n: u64 }, V2 { n: u64 } }
    public fun set_ph<T>(x: &mut Ph<T>) { x.v = 3 }
    public fun set_nested(x: &mut G<G<u64>>) { x.t.n = 4 }
    public fun set_nested_gen<T: drop>(x: &mut G<G<T>>, t: T) { x.t.t = t }

    fun c_ph(s: &mut Ph<bool>) { apply_to(s, set_ph<bool>) }
    spec c_ph { aborts_if s is B; ensures s == Ph::A<bool> { v: 3 }; }
    fun c_ph_incorrect(s: &mut Ph<bool>) { apply_to(s, set_ph<bool>) }
    spec c_ph_incorrect { ensures s == old(s); }
    fun c_ph_gen<T>(s: &mut Ph<T>) { apply_to(s, set_ph<T>) }
    spec c_ph_gen { aborts_if s is B; ensures s.v == 3; }
    fun c_ph_gen_incorrect<T>(s: &mut Ph<T>) { apply_to(s, set_ph<T>) }
    spec c_ph_gen_incorrect { ensures s == old(s); }

    fun c_nested(s: &mut G<G<u64>>) { apply_to(s, set_nested) }
    spec c_nested {
        aborts_if !(s is V1);
        ensures s.t.n == 4;
        ensures (old(s.t) is V1) ==> s.t.t == old(s.t.t);
        ensures s.n == old(s.n);
    }
    fun c_nested_incorrect(s: &mut G<G<u64>>) { apply_to(s, set_nested) }
    spec c_nested_incorrect { ensures s == old(s); }

    fun c_ng(s: &mut G<G<u8>>) { apply_to(s, |x| set_nested_gen(x, 6u8)) }
    spec c_ng { aborts_if !(s is V1) || !(s.t is V1); ensures s.t.t == 6 && s.t.n == old(s.t.n) && s.n == old(s.n); }
    fun c_ng_incorrect(s: &mut G<G<u8>>) { apply_to(s, |x| set_nested_gen(x, 6u8)) }
    spec c_ng_incorrect { aborts_if !(s is V1); }
}

// exclude_for: lean
// The Lean backend does not verify the higher-order function `apply_to` itself and does not
// interpret function-value calls across modules, with or without the variant writes.
