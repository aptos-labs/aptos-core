// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A struct invariant over a function-valued field must not be strengthened when two
// `ensures_of` premises share a result variable.
//
// The lift replaces each `ensures_of<f>(inputs, r)` premise by substituting `r` with a
// deterministic witness `f(inputs)`, one substitution per variable. With the same `r` in two
// premises the second substitution overwrote the first, so the equality `f(x) == f(y)` the
// two premises together imply was dropped and the axiom became strictly stronger than the
// invariant. Such an invariant is now not lifted.

module 0x42::behavior_axiom_shared_result {

    struct B has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec B {
        // If `f` takes the same value at two different inputs, that value is 0.
        invariant forall x: u64, y: u64, r: u64:
            x != y && ensures_of<f>(x, r) && ensures_of<f>(y, r) ==> r == 0;
    }

    /// Must fail: `B { f: identity }` satisfies the invariant, since identity never takes
    /// the same value twice, and returns 20.
    public fun shared_result(b: &B): u64 {
        (b.f)(0);
        (b.f)(20)
    }
    spec shared_result {
        ensures result == 0;
    }

    struct M has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec M {
        invariant forall x: u64: !aborts_of<f>(x);
        // Two premises with *distinct* result variables are still lifted.
        invariant forall x: u64, y: u64, r: u64, s: u64:
            x <= y && ensures_of<f>(x, r) && ensures_of<f>(y, s) ==> r <= s;
    }

    /// Must verify: monotonicity at 5 and 9.
    public fun distinct_results(m: &M): bool {
        (m.f)(5) <= (m.f)(9)
    }
    spec distinct_results {
        ensures result == true;
    }
}
