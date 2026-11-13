// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A state quantifier over a behavioral predicate of a function value must bind
// the memories that the predicate's evaluator reads at the quantified state.
// Those memories are the union of the frames of the function type's variants
// (closures, framed parameters, framed fields); a type with none binds nothing.
module 0x42::state_domain_fun_value_memory {
    struct Counter has key { value: u64 }
    struct Acc has copy, drop { value: u64 }
    struct Cb has copy, drop { g: |address| u64 has copy + drop }
    spec Cb {
        reads_of<g> Counter;
    }

    // =========================================================================
    // Parameter, reads Counter
    // =========================================================================

    fun lift_false(_f: |address| u64 has drop, _a: address) {}
    spec lift_false {
        reads_of<_f> Counter;
        requires !aborts_of<_f>(_a);
        ensures forall S in *: S |~ !aborts_of<_f>(_a); // error: post-condition does not hold
    }

    fun lower_true(_f: |address| u64 has drop, _a: address) {}
    spec lower_true {
        reads_of<_f> Counter;
        requires forall S in *: S |~ !aborts_of<_f>(_a);
        ensures !aborts_of<_f>(_a);
    }

    fun exists_true(_f: |address| u64 has drop, _a: address) {}
    spec exists_true {
        reads_of<_f> Counter;
        requires !aborts_of<_f>(_a);
        ensures exists S in *: S |~ !aborts_of<_f>(_a);
    }

    fun exists_false(_f: |address| u64 has drop, _a: address) {}
    spec exists_false {
        reads_of<_f> Counter;
        ensures exists S in *: S |~ !aborts_of<_f>(_a); // error: post-condition does not hold
    }

    fun exists_req_false(_f: |address| u64 has drop, _a: address) {}
    spec exists_req_false {
        reads_of<_f> Counter;
        requires exists S in *: S |~ !aborts_of<_f>(_a);
        ensures !aborts_of<_f>(_a); // error: post-condition does not hold
    }

    fun nested_true(_f: |address| u64 has drop, _a: address) {}
    spec nested_true {
        reads_of<_f> Counter;
        requires forall S in *: forall T in *: (S |~ aborts_of<_f>(_a)) == (T |~ aborts_of<_f>(_a));
        requires !aborts_of<_f>(_a);
        ensures forall S in *: S |~ !aborts_of<_f>(_a);
    }

    fun nested_false(_f: |address| u64 has drop, _a: address) {}
    spec nested_false {
        reads_of<_f> Counter;
        requires !aborts_of<_f>(_a);
        ensures forall S in *: forall T in *: (S |~ aborts_of<_f>(_a)) == (T |~ aborts_of<_f>(_a)); // error: post-condition does not hold
    }

    // =========================================================================
    // Framed field, and a spec let of it
    // =========================================================================

    fun field_lift_false(_c: Cb, _a: address) {}
    spec field_lift_false {
        requires !aborts_of<_c.g>(_a);
        ensures forall S in *: S |~ !aborts_of<_c.g>(_a); // error: post-condition does not hold
    }

    fun field_lower_true(_c: Cb, _a: address) {}
    spec field_lower_true {
        requires forall S in *: S |~ !aborts_of<_c.g>(_a);
        ensures !aborts_of<_c.g>(_a);
    }

    fun local_lift_false(_c: Cb, _a: address) {}
    spec local_lift_false {
        let g = _c.g;
        requires !aborts_of<g>(_a);
        ensures forall S in *: S |~ !aborts_of<g>(_a); // error: post-condition does not hold
    }

    fun local_lower_true(_c: Cb, _a: address) {}
    spec local_lower_true {
        let g = _c.g;
        requires forall S in *: S |~ !aborts_of<g>(_a);
        ensures !aborts_of<g>(_a);
    }

    // =========================================================================
    // Spec function body
    // =========================================================================

    spec fun never_aborts(f: |address| u64, a: address): bool {
        forall S in *: S |~ !aborts_of<f>(a)
    }

    fun spec_fun_lift_false(_f: |address| u64 has drop, _a: address) {}
    spec spec_fun_lift_false {
        reads_of<_f> Counter;
        requires !aborts_of<_f>(_a);
        ensures never_aborts(_f, _a); // error: post-condition does not hold
    }

    fun spec_fun_lower_true(_f: |address| u64 has drop, _a: address) {}
    spec spec_fun_lower_true {
        reads_of<_f> Counter;
        requires never_aborts(_f, _a);
        ensures !aborts_of<_f>(_a);
    }

    // =========================================================================
    // Parameter, modifies Counter: the pre and the post slot are distinct
    // =========================================================================

    fun pre_lift_false(_h: |address| bool has drop, _a: address) {}
    spec pre_lift_false {
        modifies_of<_h>(x: address) Counter[x];
        requires result_of<_h>(_a);
        ensures forall S in *: S.. |~ result_of<_h>(_a); // error: post-condition does not hold
    }

    fun pre_lower_true(_h: |address| bool has drop, _a: address) {}
    spec pre_lower_true {
        modifies_of<_h>(x: address) Counter[x];
        requires forall S in *: S.. |~ result_of<_h>(_a);
        ensures result_of<_h>(_a);
    }

    // `..S |~` puts S in the post slot; a post label also needs a pre-state use.
    fun post_lift_false(_h: |address| bool has drop, _a: address) {}
    spec post_lift_false {
        modifies_of<_h>(x: address) Counter[x];
        requires result_of<_h>(_a);
        ensures forall S in *: (..S |~ result_of<_h>(_a)) || (S.. |~ result_of<_h>(_a)); // error: post-condition does not hold
    }

    fun post_lower_true(_h: |address| bool has drop, _a: address) {}
    spec post_lower_true {
        modifies_of<_h>(x: address) Counter[x];
        requires forall S in *: (..S |~ result_of<_h>(_a)) && (S.. |~ result_of<_h>(_a));
        ensures result_of<_h>(_a);
    }

    // =========================================================================
    // No memory: the quantifier binds nothing and is dropped
    // =========================================================================

    fun nomem_lower_true(_k: |u64| u64 has drop, _x: u64) {}
    spec nomem_lower_true {
        requires forall S in *: S |~ !aborts_of<_k>(_x);
        ensures !aborts_of<_k>(_x);
    }

    fun nomem_lift_true(_k: |u64| u64 has drop, _x: u64) {}
    spec nomem_lift_true {
        requires !aborts_of<_k>(_x);
        ensures forall S in *: S |~ !aborts_of<_k>(_x);
    }

    fun nomem_false(_k: |u64| u64 has drop, _x: u64) {}
    spec nomem_false {
        ensures forall S in *: S |~ !aborts_of<_k>(_x); // error: post-condition does not hold
    }

    // =========================================================================
    // Data invariant over a framed field, checked at pack
    // =========================================================================

    struct Bounded has drop { f: |u64, u64| u64 has copy + drop }
    spec Bounded {
        reads_of<f> Counter;
        invariant forall S in *, x: u64, y: u64: S.. |~ result_of<self.f>(x, y) <= x;
    }

    fun keep(x: u64, _y: u64): u64 { x }
    spec keep {
        pragma opaque;
        aborts_if false;
        ensures result == x;
    }

    fun bump(x: u64, _y: u64): u64 { x + 1 }
    spec bump {
        pragma opaque;
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun pack_true(): Bounded {
        Bounded { f: keep }
    }

    fun pack_false(): Bounded {
        Bounded { f: bump } // error: data invariant does not hold
    }

    // =========================================================================
    // `&mut` value state together with a memory state
    // =========================================================================

    fun mixed_lower_true(_m: |&mut Acc, address| has drop, _x: &mut Acc, _a: address) {}
    spec mixed_lower_true {
        reads_of<_m> Counter;
        requires forall S in *: S.. |~ !aborts_of<_m>(_x, _a);
        ensures !aborts_of<_m>(_x, _a);
    }

    fun mixed_forall_false(_m: |&mut Acc, address| has drop, _x: &mut Acc, _a: address) {}
    spec mixed_forall_false {
        reads_of<_m> Counter;
        requires !aborts_of<_m>(_x, _a);
        ensures forall S in *: S.. |~ !aborts_of<_m>(_x, _a); // error: post-condition does not hold
    }
}
