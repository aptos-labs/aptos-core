// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A struct invariant over a function-valued field must not be assumed outside its
// quantifier's domain.
//
// Behavioral-predicate invariants are lifted into Boogie axioms that quantify each variable
// over its whole Boogie type. The lift ignored the quantifier's domain, so
// `forall x in 0..10: ensures_of<f>(x, r) ==> r == 0` became an axiom for every `x`, and a
// claim about `f` outside `0..10` was proved. `M { f: |x| if (x < 10) 0 else x }` satisfies
// the invariant and returns 20 for `x = 20`.
//
// A `where` clause restricts the domain the same way and was dropped the same way. An
// invariant whose quantifier ranges over anything narrower than a whole type, or carries a
// `where` clause, is now not lifted at all. That is sound but conservative: `in_range` is true of every value the
// invariant admits, yet is no longer provable. It is pinned here so that a later change that
// restores precision, by lifting the domain as a premise, shows up as an improvement.

module 0x42::behavior_axiom_quantifier_domain {

    struct M has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec M {
        invariant forall x in 0..10, r: u64: ensures_of<f>(x, r) ==> r == 0;
    }

    /// Must fail: 20 is outside the invariant's domain.
    public fun out_of_range(m: &M): u64 {
        (m.f)(20)
    }
    spec out_of_range {
        ensures result == 0;
    }

    /// True, but currently not provable: the invariant is no longer lifted.
    public fun in_range(m: &M): u64 {
        (m.f)(5)
    }
    spec in_range {
        ensures result == 0;
    }

    struct T has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec T {
        invariant forall x: u64, r: u64: ensures_of<f>(x, r) ==> r >= x;
    }

    /// Must verify: a whole-type quantifier is still lifted.
    public fun whole_type(t: &T): u64 {
        (t.f)(20)
    }
    spec whole_type {
        ensures result >= 20;
    }

    struct W has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec W {
        invariant forall x: u64, r: u64 where x < 10: ensures_of<f>(x, r) ==> r == 0;
    }

    /// Must fail: 20 does not satisfy the `where` clause.
    public fun out_of_where(w: &W): u64 {
        (w.f)(20)
    }
    spec out_of_where {
        ensures result == 0;
    }
}
