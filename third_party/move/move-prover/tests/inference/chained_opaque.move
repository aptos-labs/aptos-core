// Test that chaining two opaque behavior-predicate calls does not produce
// a state-label expression (..S1 |~ result_of<f>(...)) as a direct argument
// to another result_of<g>(...), which causes a panic in the Boogie backend.
//
// f1 and f2 are opaque and call an impure helper (one with a while loop /
// Assign in its body). FunctionPurenessChecker's transitive DFS sees the
// Assign inside helper and marks f1/f2 as impure → try_as_pure_spec_call
// returns None → both become behavior predicates in chain's WP.
//
// The WP should produce valid spec syntax for the composed call.
module 0x42::chained_opaque {
    struct Pool has copy, drop { value: u64 }

    // A helper with a while loop — impure under spec semantics (Assign).
    // This makes any caller of `helper` also impure (purity check is transitive).
    // Loop invariant `i <= n` lets the prover verify `result == n` without
    // inlining an unbounded loop.
    fun helper(n: u64): u64 {
        let i = 0;
        while ({
            spec { invariant i <= n; };
            i < n
        }) {
            i = i + 1;
        };
        i
    }
    spec helper {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures result == n;
    }

    // f1 and f2 call `helper`, making them transitively impure.
    // try_as_pure_spec_call returns None for both → they become behavior predicates.
    fun f1(self: &Pool, x: u64): u64 { helper(x) }
    spec f1 {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures result == x;
    }

    fun f2(self: &Pool, x: u64): u64 { helper(x) }
    spec f2 {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures result == x;
    }

    // Chains f1 then passes its result via a local to f2.
    // Both are behavior predicates; the intermediate local forces the WP to
    // process the calls separately, producing `..S1 |~ result_of<f1>(...)` as
    // an argument to `result_of<f2>(...)` — invalid spec syntax before the fix.
    fun chain(self: &Pool, x: u64): u64 {
        let r1 = f1(self, x);
        f2(self, r1)
    }
}
