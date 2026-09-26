// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A struct invariant about one function-valued field must constrain that field only.
//
// Behavioral-predicate axioms for a struct invariant are emitted once per function-valued
// field, named after that field's witness functions. The lift never checked which field a
// predicate call such as `ensures_of<f>(..)` actually targets, so with two such fields every
// invariant was emitted under both names: an invariant about `f` also constrained `g`, and
// a postcondition that holds only for `f` was proved for `g`.
//
// Each field here carries its own invariant, and the tests check both directions: each field
// keeps its own property, and neither field's property reaches the other. The generic cases
// check that a target must be this struct *instantiation*: an invariant on `G<T>` naming the
// field of a `G<u64>` value must not constrain `G<bool>`'s own field.

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
}
