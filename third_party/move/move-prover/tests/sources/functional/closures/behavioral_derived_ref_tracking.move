// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The derived contract of a function without a specification tracks the
// places references denote, for structs and enums alike. Where a write goes
// through a reference that cannot be tracked, the contract is not derived.
module 0x42::hof {
    public fun apply_to<T>(s: &mut T, f: |&mut T|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }
}

module 0x42::st {
    use 0x42::hof::apply_to;
    struct E has copy, drop, store { n: u64, k: u64 }
    fun nref(e: &mut E): &mut u64 { &mut e.n }
    fun replace(a: &mut u64, v: u64): u64 { let t = *a; *a = v; t }
    fun fields(e: &E): (&u64, &u64) { (&e.n, &e.k) }

    // A write through a reference bound by destructuring: the contract is not derived.
    public fun destr(e: &mut E) { let E { n, k: _ } = e; *n = 1 }
    fun c_destr_incorrect(s: &mut E) { apply_to(s, destr) }
    spec c_destr_incorrect { ensures s == old(s); }

    public fun destr_read(e: &mut E) { let E { n, k: _ } = e; let v = *n; e.k = v }
    fun c_destr_read(s: &mut E) { apply_to(s, destr_read) }
    spec c_destr_read { aborts_if false; ensures s.k == old(s.n) && s.n == old(s.n); }
    fun c_destr_read_incorrect(s: &mut E) { apply_to(s, destr_read) }
    spec c_destr_read_incorrect { ensures s.k == old(s.k); }

    // A write through a reference bound by matching: the contract is not derived.
    public fun matched(e: &mut E) { match (e) { E { n, k: _ } => *n = 1 } }
    fun c_matched_incorrect(s: &mut E) { apply_to(s, matched) }
    spec c_matched_incorrect { ensures s == old(s); }

    // A write through a reference chosen by a conditional: the contract is not derived.
    public fun pick(e: &mut E, c: bool) { let r = if (c) { &mut e.n } else { &mut e.k }; *r = 1 }
    fun c_pick_incorrect(s: &mut E, c: bool) { apply_to(s, |x| pick(x, c)) }
    spec c_pick_incorrect { ensures s == old(s); }

    public fun pick_read(e: &mut E) { let r = if (e.k > 0) { &mut e.n } else { &mut e.k }; let v = *r; e.k = v }
    fun c_pick_read(s: &mut E) { apply_to(s, pick_read) }
    spec c_pick_read {
        aborts_if false;
        ensures old(s.k) > 0 ==> s.k == old(s.n);
        ensures old(s.k) == 0 ==> s.k == 0;
        ensures s.n == old(s.n);
    }
    fun c_pick_read_incorrect(s: &mut E) { apply_to(s, pick_read) }
    spec c_pick_read_incorrect { ensures s.k == old(s.k); }

    public fun moved(e: &mut E) { let p = &mut e.n; let q = move p; *q = 1 }
    fun c_moved(s: &mut E) { apply_to(s, moved) }
    spec c_moved { aborts_if false; ensures s.n == 1 && s.k == old(s.k); }
    fun c_moved_incorrect(s: &mut E) { apply_to(s, moved) }
    spec c_moved_incorrect { ensures s == old(s); }

    public fun whole(e: &mut E) { let r = e; r.n = 1; r.k = 2 }
    fun c_whole(s: &mut E) { apply_to(s, whole) }
    spec c_whole { aborts_if false; ensures s.n == 1 && s.k == 2; }
    fun c_whole_incorrect(s: &mut E) { apply_to(s, whole) }
    spec c_whole_incorrect { ensures s == old(s); }

    public fun rebind(e: &mut E) { let r = &mut e.n; let r2 = r; *r2 = 7 }
    fun c_rebind(s: &mut E) { apply_to(s, rebind) }
    spec c_rebind { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_rebind_incorrect(s: &mut E) { apply_to(s, rebind) }
    spec c_rebind_incorrect { ensures s == old(s); }

    public fun reborrow(e: &mut E) { let r = &mut *e; r.n = 7 }
    fun c_reborrow(s: &mut E) { apply_to(s, reborrow) }
    spec c_reborrow { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_reborrow_incorrect(s: &mut E) { apply_to(s, reborrow) }
    spec c_reborrow_incorrect { ensures s == old(s); }

    public fun tuple(e: &mut E) { let (a, b) = (&mut e.n, 5); *a = b }
    fun c_tuple(s: &mut E) { apply_to(s, tuple) }
    spec c_tuple { aborts_if false; ensures s.n == 5 && s.k == old(s.k); }
    fun c_tuple_incorrect(s: &mut E) { apply_to(s, tuple) }
    spec c_tuple_incorrect { ensures s == old(s); }

    public fun callee(e: &mut E) { *nref(e) = 7 }
    fun c_callee(s: &mut E) { apply_to(s, callee) }
    spec c_callee { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_callee_incorrect(s: &mut E) { apply_to(s, callee) }
    spec c_callee_incorrect { ensures s == old(s); }

    public fun callee_let(e: &mut E) { let r = nref(e); *r = 7 }
    fun c_callee_let(s: &mut E) { apply_to(s, callee_let) }
    spec c_callee_let { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_callee_let_incorrect(s: &mut E) { apply_to(s, callee_let) }
    spec c_callee_let_incorrect { ensures s == old(s); }

    // A tuple of references returned by a callee.
    fun first(e: &E): u64 { let (a, _) = fields(e); *a }
    fun check_first(e: &E) {}
    spec check_first { ensures result_of<first>(e) == e.n; }
    fun check_first_incorrect(e: &E) {}
    spec check_first_incorrect { ensures result_of<first>(e) == e.k + 1; }

    // A reference assigned on one branch only.
    fun join_read(b: bool, x: u64): u64 { let r = &x; if (b) { r = &x; }; *r }
    fun check_join_read(b: bool, x: u64) {}
    spec check_join_read { ensures result_of<join_read>(b, x) == x; }
    fun check_join_read_incorrect(b: bool, x: u64) {}
    spec check_join_read_incorrect { ensures result_of<join_read>(b, x) == x + 1; }

    // The field is read before the write.
    public fun late(e: &mut E) { let a = e.n; e.n = 7; e.k = a }
    fun c_late(s: &mut E) { apply_to(s, late) }
    spec c_late { aborts_if false; ensures s.n == 7 && s.k == old(s.n); }
    fun c_late_incorrect(s: &mut E) { apply_to(s, late) }
    spec c_late_incorrect { ensures s.k == 7; }

    public fun swap(e: &mut E) { let a = e.n; let b = e.k; (a, b) = (b, a); e.n = a; e.k = b }
    fun c_swap(s: &mut E) { apply_to(s, swap) }
    spec c_swap { aborts_if false; ensures s.n == old(s.k) && s.k == old(s.n); }
    fun c_swap_incorrect(s: &mut E) { apply_to(s, swap) }
    spec c_swap_incorrect { ensures s == old(s); }

    public fun swap_ref(e: &mut E) { let t = e.n; let r = &mut e.k; let k = replace(r, t); e.n = k }
    fun c_swap_ref(s: &mut E) { apply_to(s, swap_ref) }
    spec c_swap_ref { aborts_if false; ensures s.n == old(s.k) && s.k == old(s.n); }
    fun c_swap_ref_incorrect(s: &mut E) { apply_to(s, swap_ref) }
    spec c_swap_ref_incorrect { ensures s == old(s); }
}

module 0x42::en {
    use 0x42::hof::apply_to;
    enum E has copy, drop, store { A { n: u64, k: u64 } }
    fun nref(e: &mut E): &mut u64 { &mut e.n }
    fun replace(a: &mut u64, v: u64): u64 { let t = *a; *a = v; t }

    // A write through a reference bound by destructuring: the contract is not derived.
    public fun destr(e: &mut E) { let E::A { n, k: _ } = e; *n = 1 }
    fun c_destr_incorrect(s: &mut E) { apply_to(s, destr) }
    spec c_destr_incorrect { ensures s == old(s); }

    public fun destr_read(e: &mut E) { let E::A { n, k: _ } = e; let v = *n; e.k = v }
    fun c_destr_read(s: &mut E) { apply_to(s, destr_read) }
    spec c_destr_read { aborts_if false; ensures s.k == old(s.n) && s.n == old(s.n); }
    fun c_destr_read_incorrect(s: &mut E) { apply_to(s, destr_read) }
    spec c_destr_read_incorrect { ensures s.k == old(s.k); }

    // A write through a reference bound by matching: the contract is not derived.
    public fun matched(e: &mut E) { match (e) { E::A { n, k: _ } => *n = 1 } }
    fun c_matched_incorrect(s: &mut E) { apply_to(s, matched) }
    spec c_matched_incorrect { ensures s == old(s); }

    // A write through a reference chosen by a conditional: the contract is not derived.
    public fun pick(e: &mut E, c: bool) { let r = if (c) { &mut e.n } else { &mut e.k }; *r = 1 }
    fun c_pick_incorrect(s: &mut E, c: bool) { apply_to(s, |x| pick(x, c)) }
    spec c_pick_incorrect { ensures s == old(s); }

    public fun pick_read(e: &mut E) { let r = if (e.k > 0) { &mut e.n } else { &mut e.k }; let v = *r; e.k = v }
    fun c_pick_read(s: &mut E) { apply_to(s, pick_read) }
    spec c_pick_read {
        aborts_if false;
        ensures old(s.k) > 0 ==> s.k == old(s.n);
        ensures old(s.k) == 0 ==> s.k == 0;
        ensures s.n == old(s.n);
    }
    fun c_pick_read_incorrect(s: &mut E) { apply_to(s, pick_read) }
    spec c_pick_read_incorrect { ensures s.k == old(s.k); }

    public fun moved(e: &mut E) { let p = &mut e.n; let q = move p; *q = 1 }
    fun c_moved(s: &mut E) { apply_to(s, moved) }
    spec c_moved { aborts_if false; ensures s.n == 1 && s.k == old(s.k); }
    fun c_moved_incorrect(s: &mut E) { apply_to(s, moved) }
    spec c_moved_incorrect { ensures s == old(s); }

    public fun whole(e: &mut E) { let r = e; r.n = 1; r.k = 2 }
    fun c_whole(s: &mut E) { apply_to(s, whole) }
    spec c_whole { aborts_if false; ensures s.n == 1 && s.k == 2; }
    fun c_whole_incorrect(s: &mut E) { apply_to(s, whole) }
    spec c_whole_incorrect { ensures s == old(s); }

    public fun rebind(e: &mut E) { let r = &mut e.n; let r2 = r; *r2 = 7 }
    fun c_rebind(s: &mut E) { apply_to(s, rebind) }
    spec c_rebind { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_rebind_incorrect(s: &mut E) { apply_to(s, rebind) }
    spec c_rebind_incorrect { ensures s == old(s); }

    public fun reborrow(e: &mut E) { let r = &mut *e; r.n = 7 }
    fun c_reborrow(s: &mut E) { apply_to(s, reborrow) }
    spec c_reborrow { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_reborrow_incorrect(s: &mut E) { apply_to(s, reborrow) }
    spec c_reborrow_incorrect { ensures s == old(s); }

    public fun tuple(e: &mut E) { let (a, b) = (&mut e.n, 5); *a = b }
    fun c_tuple(s: &mut E) { apply_to(s, tuple) }
    spec c_tuple { aborts_if false; ensures s.n == 5 && s.k == old(s.k); }
    fun c_tuple_incorrect(s: &mut E) { apply_to(s, tuple) }
    spec c_tuple_incorrect { ensures s == old(s); }

    public fun callee(e: &mut E) { *nref(e) = 7 }
    fun c_callee(s: &mut E) { apply_to(s, callee) }
    spec c_callee { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_callee_incorrect(s: &mut E) { apply_to(s, callee) }
    spec c_callee_incorrect { ensures s == old(s); }

    public fun callee_let(e: &mut E) { let r = nref(e); *r = 7 }
    fun c_callee_let(s: &mut E) { apply_to(s, callee_let) }
    spec c_callee_let { aborts_if false; ensures s.n == 7 && s.k == old(s.k); }
    fun c_callee_let_incorrect(s: &mut E) { apply_to(s, callee_let) }
    spec c_callee_let_incorrect { ensures s == old(s); }

    // The field is read before the write.
    public fun late(e: &mut E) { let a = e.n; e.n = 7; e.k = a }
    fun c_late(s: &mut E) { apply_to(s, late) }
    spec c_late { aborts_if false; ensures s.n == 7 && s.k == old(s.n); }
    fun c_late_incorrect(s: &mut E) { apply_to(s, late) }
    spec c_late_incorrect { ensures s.k == 7; }

    // An immutable borrow of a field keeps the value at the borrow across a shadowing of its base.
    fun same(): bool { let e = E::A { n: 1, k: 0 }; let p = &e.n; let e = E::A { n: 2, k: 0 }; let q = &e.n; *p == *q }
    fun check_same() {}
    spec check_same { ensures result_of<same>() == false; }
    fun check_same_incorrect() {}
    spec check_same_incorrect { ensures result_of<same>() == true; }

    public fun swap(e: &mut E) { let a = e.n; let b = e.k; (a, b) = (b, a); e.n = a; e.k = b }
    fun c_swap(s: &mut E) { apply_to(s, swap) }
    spec c_swap { aborts_if false; ensures s.n == old(s.k) && s.k == old(s.n); }
    fun c_swap_incorrect(s: &mut E) { apply_to(s, swap) }
    spec c_swap_incorrect { ensures s == old(s); }

    public fun swap_ref(e: &mut E) { let t = e.n; let r = &mut e.k; let k = replace(r, t); e.n = k }
    fun c_swap_ref(s: &mut E) { apply_to(s, swap_ref) }
    spec c_swap_ref { aborts_if false; ensures s.n == old(s.k) && s.k == old(s.n); }
    fun c_swap_ref_incorrect(s: &mut E) { apply_to(s, swap_ref) }
    spec c_swap_ref_incorrect { ensures s == old(s); }
}

// Shapes that stay derived: inline functions, reference-returning helpers,
// immutable borrows, reassigned references.
module 0x42::kept {
    use std::vector;
    use 0x42::hof::apply_to;
    struct T has copy, drop, store { f: u64, g: u64 }
    struct V has copy, drop, store { v: vector<u64>, k: u64 }

    inline fun inc_f(t: &mut T) { t.f = t.f + 1 }
    public fun inl(x: &mut T) { inc_f(x) }
    fun c_inl(s: &mut T) { apply_to(s, inl) }
    spec c_inl { aborts_if s.f + 1 > MAX_U64; ensures s.f == old(s.f) + 1; ensures s.g == old(s.g); }
    fun c_inl_incorrect(s: &mut T) { apply_to(s, inl) }
    spec c_inl_incorrect { ensures s == old(s); }

    inline fun inc_self(self: &mut T) { self.f = self.f + 1 }
    public fun inl_self(x: &mut T) { x.inc_self() }
    fun c_inl_self(s: &mut T) { apply_to(s, inl_self) }
    spec c_inl_self { aborts_if s.f + 1 > MAX_U64; ensures s.f == old(s.f) + 1; ensures s.g == old(s.g); }
    fun c_inl_self_incorrect(s: &mut T) { apply_to(s, inl_self) }
    spec c_inl_self_incorrect { ensures s == old(s); }

    inline fun app(s: &mut T, f: |&mut T|) { f(s) }
    public fun inl_lambda(x: &mut T) { app(x, |t| t.f = 5) }
    fun c_inl_lambda(s: &mut T) { apply_to(s, inl_lambda) }
    spec c_inl_lambda { aborts_if false; ensures s.f == 5; ensures s.g == old(s.g); }
    fun c_inl_lambda_incorrect(s: &mut T) { apply_to(s, inl_lambda) }
    spec c_inl_lambda_incorrect { ensures s == old(s); }

    inline fun fref1(t: &mut T): &mut u64 { &mut t.f }
    public fun inl_ref_ret(x: &mut T) { let r = fref1(x); *r = 7 }
    fun c_inl_ref_ret(s: &mut T) { apply_to(s, inl_ref_ret) }
    spec c_inl_ref_ret { aborts_if false; ensures s.f == 7; ensures s.g == old(s.g); }
    fun c_inl_ref_ret_incorrect(s: &mut T) { apply_to(s, inl_ref_ret) }
    spec c_inl_ref_ret_incorrect { ensures s == old(s); }

    inline fun fref(t: &mut T, _k: u64): &mut u64 { &mut t.f }
    public fun inl_ref_ret2(x: &mut T) { let r = fref(x, 0); *r = 7 }
    fun c_inl_ref_ret2(s: &mut T) { apply_to(s, inl_ref_ret2) }
    spec c_inl_ref_ret2 { aborts_if false; ensures s.f == 7; ensures s.g == old(s.g); }
    fun c_inl_ref_ret2_incorrect(s: &mut T) { apply_to(s, inl_ref_ret2) }
    spec c_inl_ref_ret2_incorrect { ensures s == old(s); }

    inline fun push7(v: &mut vector<u64>) { vector::push_back(v, 7) }
    public fun inl_vec(x: &mut V) { push7(&mut x.v) }
    fun c_inl_vec(s: &mut V) { apply_to(s, inl_vec) }
    spec c_inl_vec { aborts_if false; ensures s.v == concat(old(s.v), vec(7)); ensures s.k == old(s.k); }
    fun c_inl_vec_incorrect(s: &mut V) { apply_to(s, inl_vec) }
    spec c_inl_vec_incorrect { ensures s == old(s); }

    // The inline parameter is named like a caller local a dead reference is rooted at.
    inline fun add2(e: &mut u64, k: u64) { *e = *e + k }
    public fun inl_param(x: &mut T) { let e = T { f: 0, g: 0 }; let r = &mut e.g; *r = 1; add2(&mut x.f, e.g) }
    fun c_inl_param(s: &mut T) { apply_to(s, inl_param) }
    spec c_inl_param { aborts_if s.f + 1 > MAX_U64; ensures s.f == old(s.f) + 1; ensures s.g == old(s.g); }
    fun c_inl_param_incorrect(s: &mut T) { apply_to(s, inl_param) }
    spec c_inl_param_incorrect { ensures s == old(s); }

    inline fun set_two(a: &mut u64, b: &mut u64) { *a = 5; *b = 6 }
    public fun tuple_inl(x: &mut T) { let (p, q) = (&mut x.f, &mut x.g); set_two(p, q) }
    fun c_tuple_inl(s: &mut T) { apply_to(s, tuple_inl) }
    spec c_tuple_inl { aborts_if false; ensures s.f == 5; ensures s.g == 6; }
    fun c_tuple_inl_incorrect(s: &mut T) { apply_to(s, tuple_inl) }
    spec c_tuple_inl_incorrect { ensures s == old(s); }

    fun get(t: &T): &u64 { &t.f }
    public fun ref_helper(x: &mut T) { let v = *get(x); x.g = v }
    fun c_ref_helper(s: &mut T) { apply_to(s, ref_helper) }
    spec c_ref_helper { aborts_if false; ensures s.g == old(s.f); ensures s.f == old(s.f); }
    fun c_ref_helper_incorrect(s: &mut T) { apply_to(s, ref_helper) }
    spec c_ref_helper_incorrect { ensures s == old(s); }

    fun kv(t: &T): (&u64, &u64) { (&t.f, &t.g) }
    public fun ref_helper_tuple(x: &mut T) { let (a, b) = kv(x); let n = *a + *b; x.f = n }
    fun c_ref_helper_tuple(s: &mut T) { apply_to(s, ref_helper_tuple) }
    spec c_ref_helper_tuple { aborts_if s.f + s.g > MAX_U64; ensures s.f == old(s.f) + old(s.g); ensures s.g == old(s.g); }
    fun c_ref_helper_tuple_incorrect(s: &mut T) { apply_to(s, ref_helper_tuple) }
    spec c_ref_helper_tuple_incorrect { ensures s == old(s); }

    fun peek(t: &mut T): &u64 { &t.f }
    public fun ref_helper_mut(x: &mut T) { let v = *peek(x); x.g = v }
    fun c_ref_helper_mut(s: &mut T) { apply_to(s, ref_helper_mut) }
    spec c_ref_helper_mut { aborts_if false; ensures s.g == old(s.f); ensures s.f == old(s.f); }
    fun c_ref_helper_mut_incorrect(s: &mut T) { apply_to(s, ref_helper_mut) }
    spec c_ref_helper_mut_incorrect { ensures s == old(s); }

    public fun vec_read(x: &mut V) {
        let n = vector::length(&x.v);
        let b = vector::contains(&x.v, &7);
        let e = if (n > 0) x.v[0] else 0;
        x.k = if (b) n else e;
    }
    fun c_vec_read(s: &mut V) { apply_to(s, vec_read) }
    spec c_vec_read {
        aborts_if false;
        ensures s.v == old(s.v);
        ensures s.k == (if (contains(old(s.v), 7)) len(old(s.v)) else if (len(old(s.v)) > 0) old(s.v)[0] else 0);
    }
    fun c_vec_read_incorrect(s: &mut V) { apply_to(s, vec_read) }
    spec c_vec_read_incorrect { ensures s == old(s); }

    public fun vec_borrow(x: &mut V) {
        let r = vector::borrow(&x.v, 0);
        let a = *r;
        x.k = a;
        let w = vector::borrow_mut(&mut x.v, 0);
        *w = a + 1;
    }
    fun c_vec_borrow(s: &mut V) { apply_to(s, vec_borrow) }
    spec c_vec_borrow {
        aborts_if len(s.v) == 0;
        aborts_if s.v[0] + 1 > MAX_U64;
        ensures len(s.v) == len(old(s.v));
        ensures s.v[0] == old(s.v)[0] + 1;
        ensures s.k == old(s.v)[0];
    }
    fun c_vec_borrow_incorrect(s: &mut V) { apply_to(s, vec_borrow) }
    spec c_vec_borrow_incorrect { ensures s == old(s); }

    public fun borrow_before_write(x: &mut T) { let n = *&x.f; x.f = 1; let r = &x.g; let m = *r; x.g = m + n }
    fun c_borrow_before_write(s: &mut T) { apply_to(s, borrow_before_write) }
    spec c_borrow_before_write { aborts_if s.f + s.g > MAX_U64; ensures s.f == 1; ensures s.g == old(s.g) + old(s.f); }
    fun c_borrow_before_write_incorrect(s: &mut T) { apply_to(s, borrow_before_write) }
    spec c_borrow_before_write_incorrect { ensures s == old(s); }

    public fun branches_imm(x: &mut T) {
        let v = if (x.f > 3) { let r = &x.f; *r } else { let r = &x.g; *r };
        let r2 = if (x.f > 10) &x.f else &x.g;
        let w = *r2;
        x.g = v; x.f = w;
    }
    fun c_branches_imm(s: &mut T) { apply_to(s, branches_imm) }
    spec c_branches_imm {
        aborts_if false;
        ensures s.g == (if (old(s.f) > 3) old(s.f) else old(s.g));
        ensures s.f == (if (old(s.f) > 10) old(s.f) else old(s.g));
    }
    fun c_branches_imm_incorrect(s: &mut T) { apply_to(s, branches_imm) }
    spec c_branches_imm_incorrect { ensures s == old(s); }

    public fun imm_then_mut(x: &mut T) { let a = &x.f; let n = *a; let b = &mut x.f; *b = n + 1 }
    fun c_imm_then_mut(s: &mut T) { apply_to(s, imm_then_mut) }
    spec c_imm_then_mut { aborts_if s.f + 1 > MAX_U64; ensures s.f == old(s.f) + 1; ensures s.g == old(s.g); }
    fun c_imm_then_mut_incorrect(s: &mut T) { apply_to(s, imm_then_mut) }
    spec c_imm_then_mut_incorrect { ensures s == old(s); }

    fun rd(t: &T): u64 { t.f }
    fun rd2(t: &T): u64 { t.f }
    spec rd2 { aborts_if false; ensures result == t.f; }
    public fun freeze_callees(x: &mut T) { let n = rd(x); let y = freeze(x); let m = rd2(y); x.g = n + m }
    fun c_freeze_callees(s: &mut T) { apply_to(s, freeze_callees) }
    spec c_freeze_callees { aborts_if s.f + s.f > MAX_U64; ensures s.g == old(s.f) + old(s.f); ensures s.f == old(s.f); }
    fun c_freeze_callees_incorrect(s: &mut T) { apply_to(s, freeze_callees) }
    spec c_freeze_callees_incorrect { ensures s == old(s); }

    public fun reassign_ref(x: &mut T) { let r = &mut x.f; *r = 1; r = &mut x.g; *r = 2 }
    fun c_reassign_ref(s: &mut T) { apply_to(s, reassign_ref) }
    spec c_reassign_ref { aborts_if false; ensures s.f == 1; ensures s.g == 2; }
    fun c_reassign_ref_incorrect(s: &mut T) { apply_to(s, reassign_ref) }
    spec c_reassign_ref_incorrect { ensures s == old(s); }

    public fun move_copy_ref(x: &mut T) { let q = &x.f; let q2 = copy q; let n = *q2; let r = &mut x.f; let r2 = move r; *r2 = 9; x.g = n }
    fun c_move_copy_ref(s: &mut T) { apply_to(s, move_copy_ref) }
    spec c_move_copy_ref { aborts_if false; ensures s.f == 9; ensures s.g == old(s.f); }
    fun c_move_copy_ref_incorrect(s: &mut T) { apply_to(s, move_copy_ref) }
    spec c_move_copy_ref_incorrect { ensures s == old(s); }

    public fun block_ref(x: &mut T) { let n = { let q = &x.f; *q }; let r = { let t = &mut x.f; t }; *r = 4; x.g = n }
    fun c_block_ref(s: &mut T) { apply_to(s, block_ref) }
    spec c_block_ref { aborts_if false; ensures s.f == 4; ensures s.g == old(s.f); }
    fun c_block_ref_incorrect(s: &mut T) { apply_to(s, block_ref) }
    spec c_block_ref_incorrect { ensures s == old(s); }

    // A local assigned again after a reference rooted at it is dead.
    public fun assign_rooted_local(x: &mut T) { let l = T { f: 0, g: 0 }; let r = &mut l.f; *r = 1; l = T { f: 2, g: 0 }; let r2 = &mut l.g; *r2 = 3; x.f = l.f }
    fun c_assign_rooted_local(s: &mut T) { apply_to(s, assign_rooted_local) }
    spec c_assign_rooted_local { aborts_if false; ensures s.f == 2; ensures s.g == old(s.g); }
    fun c_assign_rooted_local_incorrect(s: &mut T) { apply_to(s, assign_rooted_local) }
    spec c_assign_rooted_local_incorrect { ensures s == old(s); }
}

// A lambda passed to an inline function shadows its own `&mut` parameter by value.
module 0x42::lam_shadow {
    struct T has copy, drop, store { f: u64, g: u64 }
    inline fun app(s: &mut T, f: |&mut T|) { let pre = *s; f(s); spec { assert ensures_of<f>(pre, s); }; }
    inline fun app_neg(s: &mut T, f: |&mut T|) { let pre = *s; f(s); spec { assert ensures_of<f>(pre, s) ==> s == pre; }; }

    fun c_deref_shadow(x: &mut T) { app(x, |t| { let v = { let t = *t; t.f + 1 }; t.g = v }) }
    spec c_deref_shadow { pragma aborts_if_is_partial; }
    fun c_deref_shadow_incorrect(x: &mut T) { app_neg(x, |t| { let v = { let t = *t; t.f + 1 }; t.g = v }) }
    spec c_deref_shadow_incorrect { pragma aborts_if_is_partial; }
}

// A match arm binds a name a dead reference was rooted at.
module 0x42::match_dead_ref {
    use 0x42::hof::apply_to;
    struct T has copy, drop, store { f: u64, g: u64 }
    enum O has copy, drop { S { v: u64 }, N }
    public fun set(x: &mut T) { let v = 1; let r = &mut v; *r = 7; let w = match (O::S { v: x.g }) { O::S { v } => v, O::N => 0 }; x.f = v; x.g = w }
    fun c_set(s: &mut T) { apply_to(s, set) }
    spec c_set { aborts_if false; ensures s.f == 7; ensures s.g == old(s.g); }
    fun c_set_incorrect(s: &mut T) { apply_to(s, set) }
    spec c_set_incorrect { ensures s == old(s); }
}

// Lambdas passed to inline higher-order functions writing through their `&mut` parameters.
module 0x42::lam_inline {
    struct T has copy, drop, store { a: u64, b: u64 }
    enum O has copy, drop { S { v: u64 }, N }
    inline fun app(s: &mut T, f: |&mut T|) { let pre = *s; f(s); spec { assert ensures_of<f>(pre, s); }; }
    inline fun app_neg(s: &mut T, f: |&mut T|) { let pre = *s; f(s); spec { assert ensures_of<f>(pre, s) ==> s == pre; }; }
    inline fun app2(t: &mut u64, s: &mut T, f: |&mut u64, &mut T|) { let pt = *t; let pre = *s; f(t, s); spec { assert ensures_of<f>(pt, pre, t, s); }; }
    inline fun app2_neg(t: &mut u64, s: &mut T, f: |&mut u64, &mut T|) { let pt = *t; let pre = *s; f(t, s); spec { assert ensures_of<f>(pt, pre, t, s) ==> s == pre; }; }

    fun c_one(x: &mut T) { app(x, |t| { t.a = 1 }) }
    spec c_one { pragma aborts_if_is_partial; }
    fun c_one_incorrect(x: &mut T) { app_neg(x, |t| { t.a = 1 }) }
    spec c_one_incorrect { pragma aborts_if_is_partial; }

    fun c_two(y: &mut u64, x: &mut T) { app2(y, x, |t2, s2| { s2.a = 1; *t2 = 2 }) }
    spec c_two { pragma aborts_if_is_partial; }
    fun c_two_incorrect(y: &mut u64, x: &mut T) { app2_neg(y, x, |t2, s2| { s2.a = 1; *t2 = 2 }) }
    spec c_two_incorrect { pragma aborts_if_is_partial; }

    // The lambda parameter is shadowed by a value in an inner block.
    fun c_value_shadow(x: &mut T) { app(x, |t| { let w = { let t = 7u64; t }; t.a = w }) }
    spec c_value_shadow { pragma aborts_if_is_partial; }
    fun c_value_shadow_incorrect(x: &mut T) { app_neg(x, |t| { let w = { let t = 7u64; t }; t.a = w }) }
    spec c_value_shadow_incorrect { pragma aborts_if_is_partial; }

    // As above, with a live reference rooted at the parameter.
    fun c_unused_ref_shadow(x: &mut T) { app(x, |t| { let r = &mut t.a; let w = { let t = 7; t }; *r = w }) }
    spec c_unused_ref_shadow { pragma aborts_if_is_partial; }
    fun c_unused_ref_shadow_incorrect(x: &mut T) { app_neg(x, |t| { let r = &mut t.a; let w = { let t = 7; t }; *r = w }) }
    spec c_unused_ref_shadow_incorrect { pragma aborts_if_is_partial; }

    // A match arm binds the name of the lambda parameter.
    fun c_match_shadow(x: &mut T) { app(x, |t| { let o = if (t.b > 0) O::S { v: t.b } else O::N; let v = match (o) { O::S { v: t } => t, O::N => 0 }; t.a = v }) }
    spec c_match_shadow { pragma aborts_if_is_partial; }
    fun c_match_shadow_incorrect(x: &mut T) { app_neg(x, |t| { let o = if (t.b > 0) O::S { v: t.b } else O::N; let v = match (o) { O::S { v: t } => t, O::N => 0 }; t.a = v }) }
    spec c_match_shadow_incorrect { pragma aborts_if_is_partial; }
}

// exclude_for: lean
// The Lean backend does not verify the higher-order function `apply_to` itself.
