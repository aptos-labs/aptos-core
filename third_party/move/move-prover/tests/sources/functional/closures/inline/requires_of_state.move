// `requires_of` over a lambda whose attached `requires` reads global state:
// requires conditions hold at the APPLICATION's pre-state and get the same
// state treatment as abort conditions — memory reads are `old(..)`-wrapped,
// resolving at the unique application's anchor, or at function entry in a
// loop invariant. Without the pre-state treatment the reads would be
// evaluated at the assertion-site state: the wrong state once the inline
// body changed memory after applying the lambda, turning a false
// `requires_of` claim into a provable one (and a true one into a failure).
// (The rejected form — no unique application site to anchor — is in
// `requires_of_state_err.move`.)
module 0x42::requires_of_state {
    use std::vector;

    struct R has key { v: u64 }

    fun read_r(a: address): u64 acquires R {
        R[a].v
    }

    // ===== plain assert, anchored at the unique application site =====

    inline fun apply_then_bump(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        R[a].v = R[a].v + 1;
        spec {
            // Resolved at the application's pre-state via the anchor; the
            // bump after the application must not influence the claim.
            assert requires_of<f>(x); // error: false at the application for the `false_at_application` caller
        };
        res
    }

    // The lambda's `requires R[a].v > 0` holds at the application (equal to
    // the caller's entry state).
    fun true_at_application(a: address, x: u64): u64 {
        apply_then_bump(|y| y spec { requires R[a].v > 0; ensures result == y; }, a, x)
    }
    spec true_at_application {
        requires exists<R>(a) && R[a].v > 0 && R[a].v < MAX_U64;
        pragma aborts_if_is_partial;
    }

    // The lambda's `requires R[a].v > 0` is false at the application
    // (R[a].v == 0); only after the bump does R[a].v == 1 > 0 hold, so an
    // assertion-site evaluation would wrongly accept the claim.
    fun false_at_application(a: address, x: u64): u64 {
        apply_then_bump(|y| y spec { requires R[a].v > 0; ensures result == y; }, a, x)
    }
    spec false_at_application {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // The same state read can be hidden behind a default-range behavioral
    // predicate. The nested evaluator must also be moved to the application
    // pre-state; reading it after the bump would make this false claim pass.
    fun nested_false_at_application(a: address, x: u64): u64 {
        apply_then_bump(
            |y| y spec {
                requires result_of<read_r>(a) > 0;
                ensures result == y;
            },
            a,
            x,
        )
    }
    spec nested_false_at_application {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // ===== loop invariant: memory reads resolve to function entry =====

    inline fun for_each_bump(v: &vector<u64>, a: address, f: |u64| u64) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i));
            R[a].v = R[a].v + 1;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant R[a].v == old(R[a].v) + i;
            // Reads resolve to entry; the loop's own bumps must not
            // influence the claim.
            invariant forall j in 0..i: requires_of<f>(v[j]); // error: false at entry for the `false_at_entry` caller
        };
    }

    // `requires R[a].v == 0` holds at entry: the invariant verifies even
    // though the current state has R[a].v == i after i iterations (an
    // assertion-site evaluation would wrongly reject it).
    fun true_at_entry(v: &vector<u64>, a: address) {
        for_each_bump(v, a, |y| y spec { requires R[a].v == 0; ensures result == y; });
    }
    spec true_at_entry {
        requires exists<R>(a) && R[a].v == 0;
        requires len(v) < 100;
        pragma aborts_if_is_partial;
    }

    // `requires R[a].v > 0` is false at entry; after the first iteration
    // the current state has R[a].v > 0, so an assertion-site evaluation
    // would wrongly accept it.
    fun false_at_entry(v: &vector<u64>, a: address) {
        for_each_bump(v, a, |y| y spec { requires R[a].v > 0; ensures result == y; });
    }
    spec false_at_entry {
        requires exists<R>(a) && R[a].v == 0;
        requires len(v) < 100;
        pragma aborts_if_is_partial;
    }
}
