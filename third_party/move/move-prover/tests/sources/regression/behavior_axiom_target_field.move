// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A struct invariant about a function-valued field constrains only that field of the same
// struct instantiation, stated for the value itself or a universally quantified value.

module 0x42::behavior_axiom_target_field {

    struct S has key, drop {
        f: |u64|u64 has copy+store+drop,
        g: |u64|u64 has copy+store+drop,
    }
    spec S {
        invariant forall x: u64: !aborts_of<f>(x);
        invariant forall x: u64, r: u64: ensures_of<f>(x, r) ==> r >= x;
        invariant forall x: u64: !aborts_of<g>(x);
        invariant forall x: u64, r: u64: ensures_of<g>(x, r) ==> r <= x;
    }

    /// Must verify: `f`'s own invariant.
    public fun f_lower(s: &S): u64 {
        (s.f)(5)
    }
    spec f_lower {
        ensures result >= 5;
    }

    /// Must verify: `g`'s own invariant.
    public fun g_upper(s: &S): u64 {
        (s.g)(5)
    }
    spec g_upper {
        ensures result <= 5;
    }

    /// Must fail: only `f` is bounded below. `S { f: identity, g: zero }` returns 0.
    public fun g_lower(s: &S): u64 {
        (s.g)(5)
    }
    spec g_lower {
        ensures result >= 5;
    }

    /// Must fail: only `g` is bounded above. `S { f: double, g: zero }` returns 10.
    public fun f_upper(s: &S): u64 {
        (s.f)(5)
    }
    spec f_upper {
        ensures result <= 5;
    }

    // -- target must be this instantiation, not just this struct declaration ------------

    struct G<phantom T> has key, drop, copy, store {
        f: |u64|u64 has copy+store+drop,
    }
    spec G {
        // A property of every `G<u64>` value's field. It says nothing about `G<bool>`.
        invariant forall o: G<u64>, x: u64, r: u64: ensures_of<o.f>(x, r) ==> r >= x;
    }

    /// Must fail: `G<bool>`'s own `f` is unconstrained.
    public fun other_instantiation(g: &G<bool>): u64 {
        (g.f)(5)
    }
    spec other_instantiation {
        ensures result >= 5;
    }

    struct H<phantom T> has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec H {
        invariant forall x: u64, r: u64: ensures_of<f>(x, r) ==> r >= x;
    }

    /// Must verify: a generic struct's invariant on its own field holds at every instantiation.
    public fun own_field_generic(h: &H<bool>): u64 {
        (h.f)(5)
    }
    spec own_field_generic {
        ensures result >= 5;
    }

    struct N has key, drop, copy, store {
        f: |u64|u64 has copy+store+drop,
    }
    spec N {
        // Quantifies over every `N`, which includes the value itself.
        invariant forall o: N, x: u64, r: u64: ensures_of<o.f>(x, r) ==> r >= x;
    }

    /// Must verify: the target is another value, but of this exact type.
    public fun same_type_quantified(n: &N): u64 {
        (n.f)(5)
    }
    spec same_type_quantified {
        ensures result >= 5;
    }

    struct P has key, drop, copy, store {
        f: |u64|u64 has copy+store+drop,
    }

    public fun zero(_x: u64): u64 { 0 }

    spec P {
        // True of every `P`: it only mentions one fixed value's field.
        invariant forall x: u64, r: u64: ensures_of<P { f: zero }.f>(x, r) ==> r == 0;
    }

    /// Must fail: the invariant holds for `P { f: identity }`, which returns 5.
    public fun fixed_value(p: &P): u64 {
        (p.f)(5)
    }
    spec fixed_value {
        ensures result == 0;
    }

    enum E has drop, copy, store {
        V { f: |u64|u64 has copy+store+drop },
    }

    spec E {
        invariant forall x: u64: !aborts_of<self.f>(x);
        invariant forall x: u64, r: u64: ensures_of<self.f>(x, r) ==> r >= x;
    }

    /// Must verify: an enum's own field.
    public fun enum_field(e: &E): u64 {
        (e.f)(5)
    }
    spec enum_field {
        ensures result >= 5;
    }

    struct F<T> has drop, copy {
        f: |u64|u64 has copy+store+drop,
        t: T,
    }

    spec F {
        invariant forall o: F<|u64|u64 has copy+drop>, x: u64: !aborts_of<o.f>(x);
        invariant forall o: F<|u64|u64 has copy+drop>, x: u64, r: u64: ensures_of<o.f>(x, r) ==> r >= x;
    }

    /// Must verify: a quantified value at a function-type instantiation.
    public fun fun_inst(g: &F<|u64|u64 has copy+drop>): u64 {
        (g.f)(5)
    }
    spec fun_inst {
        ensures result >= 5;
    }
}
