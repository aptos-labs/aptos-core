// Pre-state conditions (`requires_of`, `aborts_of`) phrased through
// MEMORY-READING SPEC FUNCTIONS: a condition reading global state only
// indirectly — through a user spec function whose body (possibly
// transitively) reads memory — gets the same pre-state treatment as a
// direct `global`/`exists` read. The spec function call itself is
// `old(..)`-wrapped, resolving at the unique application's anchor, or at
// function entry in a loop invariant. Spec function memory usage is
// established by a transitive inline-time body scan
// (`SpecFunDecl::used_memory` is only computed by the later spec
// rewriter). Without this, the call would be evaluated at the
// assertion-site state: the wrong state once the inline body changed
// memory after applying the lambda, turning a false claim into a provable
// one. (The rejected forms — no unique application site, state-dependent
// `result_of` values — are in `bp_specfun_state_err.move`.)
module 0x42::bp_specfun_state {
    use std::vector;

    struct R has key { v: u64 }

    spec fun spec_get_v(a: address): u64 {
        global<R>(a).v
    }

    // Reads memory only transitively, through another spec function.
    spec fun spec_get_v_indirect(a: address): u64 {
        spec_get_v(a)
    }

    // ===== requires_of: plain assert, anchored at the application =====

    inline fun apply_then_bump(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        R[a].v = R[a].v + 1;
        spec {
            // Resolved at the application's pre-state via the anchor; the
            // bump after the application must not influence the claim.
            assert requires_of<f>(x); // error: false at the application for the `requires_false_*` callers
        };
        res
    }

    // The lambda's `requires spec_get_v(a) > 0` holds at the application.
    fun requires_true_at_application(a: address, x: u64): u64 {
        apply_then_bump(|y| y spec { requires spec_get_v(a) > 0; ensures result == y; }, a, x)
    }
    spec requires_true_at_application {
        requires exists<R>(a) && R[a].v > 0 && R[a].v < MAX_U64;
        pragma aborts_if_is_partial;
    }

    // The lambda's `requires spec_get_v(a) > 0` is false at the application
    // (R[a].v == 0); only after the bump does R[a].v == 1 > 0 hold, so an
    // assertion-site evaluation would wrongly accept the claim.
    fun requires_false_at_application(a: address, x: u64): u64 {
        apply_then_bump(|y| y spec { requires spec_get_v(a) > 0; ensures result == y; }, a, x)
    }
    spec requires_false_at_application {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // Same through the transitively-reading spec function.
    fun requires_false_transitive(a: address, x: u64): u64 {
        apply_then_bump(
            |y| y spec { requires spec_get_v_indirect(a) > 0; ensures result == y; }, a, x)
    }
    spec requires_false_transitive {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // ===== aborts_of: plain assert, anchored at the application =====

    inline fun apply_then_bump_na(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        R[a].v = R[a].v + 1;
        spec {
            assert !aborts_of<f>(x); // error: the abort condition holds at the application for the `aborts_at_application` caller
        };
        res
    }

    // The lambda's abort condition `spec_get_v(a) == 0` is false at the
    // application (R[a].v > 0).
    fun no_abort_at_application(a: address, x: u64): u64 {
        apply_then_bump_na(
            |y| y spec { aborts_if spec_get_v(a) == 0; ensures result == y; }, a, x)
    }
    spec no_abort_at_application {
        requires exists<R>(a) && R[a].v > 0 && R[a].v < MAX_U64;
        pragma aborts_if_is_partial;
    }

    // The abort condition holds at the application (R[a].v == 0); after the
    // bump R[a].v == 1 != 0, so an assertion-site evaluation would wrongly
    // accept `!aborts_of`.
    fun aborts_at_application(a: address, x: u64): u64 {
        apply_then_bump_na(
            |y| y spec { aborts_if spec_get_v(a) == 0; ensures result == y; }, a, x)
    }
    spec aborts_at_application {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // ===== loop invariant: the reads resolve to function entry =====

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
            invariant forall j in 0..i: requires_of<f>(v[j]); // error: false at entry for the `loop_false_at_entry` caller
        };
    }

    // `requires spec_get_v(a) == 0` holds at entry: the invariant verifies
    // even though the current state has R[a].v == i after i iterations.
    fun loop_true_at_entry(v: &vector<u64>, a: address) {
        for_each_bump(v, a, |y| y spec { requires spec_get_v(a) == 0; ensures result == y; });
    }
    spec loop_true_at_entry {
        requires exists<R>(a) && R[a].v == 0;
        requires len(v) < 100;
        pragma aborts_if_is_partial;
    }

    // `requires spec_get_v(a) > 0` is false at entry; after the first
    // iteration the current state has R[a].v > 0, so an assertion-site
    // evaluation would wrongly accept it.
    fun loop_false_at_entry(v: &vector<u64>, a: address) {
        for_each_bump(v, a, |y| y spec { requires spec_get_v(a) > 0; ensures result == y; });
    }
    spec loop_false_at_entry {
        requires exists<R>(a) && R[a].v == 0;
        requires len(v) < 100;
        pragma aborts_if_is_partial;
    }

    // Mutually recursive state usage. `recursive_f` reaches `recursive_g`
    // before its direct memory read, while `recursive_g` calls back into
    // `recursive_f`; the inline-time analysis must propagate that read to
    // both functions instead of leaving `recursive_g` state-free.
    spec fun recursive_f(a: address, n: num): bool {
        if (n <= 0) false else recursive_g(a, n - 1) || spec_get_v(a) == 1
    }

    spec fun recursive_g(a: address, n: num): bool {
        if (n <= 0) false else recursive_f(a, n - 1)
    }

    // Both sibling calls read memory through the mutual recursion. At the
    // application state (R[a].v == 0) both are false; after the inline body
    // bumps the resource both would be true if `recursive_g` were left
    // unanchored by an incomplete cycle memo.
    fun requires_false_mutual_recursive(a: address, x: u64): u64 {
        apply_then_bump(
            |y| y spec {
                requires recursive_f(a, 1) || recursive_g(a, 2);
                ensures result == y;
            },
            a,
            x,
        )
    }
    spec requires_false_mutual_recursive {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }
}
