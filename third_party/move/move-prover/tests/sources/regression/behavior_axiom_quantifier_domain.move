// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A struct invariant over a function-valued field holds only within its quantifier's domain.
// Invariants quantifying over less than a whole type, or with a `where` clause, are not
// lifted; `in_range` is true but unprovable as a result.

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

    /// True, but not provable: the invariant is not lifted.
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

    /// Must verify: a whole-type quantifier is lifted.
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
