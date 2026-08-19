// `ensures_of` over a lambda whose attached ensures calls a TWO-STATE spec
// function — one whose body uses `old(..)`. The spec function's old-state
// memory must be bound at the unique application's anchor: the two-state
// detection is a transitive inline-time body scan (`SpecFunDecl::uses_old`
// is only computed by the later spec rewriter), and the anchor wrapper
// makes the move-model spec translator save the function's old memory at
// the anchor instead of function entry. Without this, the helper's
// `old(..)` reads the ENCLOSING function's entry state — the wrong state
// once memory changed before the application, letting a false claim verify
// (and failing a true one). In a loop invariant the pre-state resolves to
// function entry, the invariant's own `old(..)` scope. (Two-state spec
// functions in `requires`/`aborts_if` are rejected, see
// `bp_specfun_state_err.move`.)
module 0x42::two_state_spec_fun {
    use std::vector;

    struct R has key { v: u64 }

    // delta between the current and the pre-state value at `a`.
    spec fun spec_delta(a: address): u64 {
        global<R>(a).v - old(global<R>(a).v)
    }

    // ===== plain assert, anchored at the unique application =====

    inline fun bump_then_apply(f: |u64| u64, a: address, x: u64): u64 {
        R[a].v = R[a].v + 1; // mutation BEFORE the unique application
        let res = f(x);
        spec {
            assert ensures_of<f>(x, res); // error: the lambda adds 2, not 3, for the `delta_false` caller
        };
        res
    }

    // TRUE claim: over the application (anchor -> post) the delta is 2.
    // An entry-bound `old(..)` would see the pre-application bump and
    // compute 3, wrongly rejecting the claim.
    fun delta_true(a: address, x: u64): u64 {
        bump_then_apply(|y| { R[a].v = R[a].v + 2; y }
            spec { ensures spec_delta(a) == 2; ensures result == y; }, a, x)
    }
    spec delta_true {
        requires exists<R>(a) && R[a].v < 1000;
        pragma aborts_if_is_partial;
    }

    // FALSE claim: the lambda adds 2, the spec claims 3. An entry-bound
    // `old(..)` would fold the pre-application bump into the delta
    // (1 + 2 == 3) and wrongly accept it.
    fun delta_false(a: address, x: u64): u64 {
        bump_then_apply(|y| { R[a].v = R[a].v + 2; y }
            spec { ensures spec_delta(a) == 3; ensures result == y; }, a, x)
    }
    spec delta_false {
        requires exists<R>(a) && R[a].v < 1000;
        pragma aborts_if_is_partial;
    }

    // ===== loop invariant: the pre-state resolves to function entry =====

    inline fun for_each_apply(v: &vector<u64>, a: address, f: |u64|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant R[a].v == old(R[a].v) + i;
            invariant forall j in 0..i: ensures_of<f>(v[j]); // error: entry delta is i, not 0, for the `loop_delta_false` caller
        };
    }

    // Entry-resolved delta after i iterations is i; the lambda's claim
    // `spec_delta(a) <= 100` holds under the caller's length bound.
    fun loop_delta_true(v: &vector<u64>, a: address) {
        for_each_apply(v, a, |_y| { R[a].v = R[a].v + 1; }
            spec { ensures spec_delta(a) <= 100; });
    }
    spec loop_delta_true {
        requires exists<R>(a) && R[a].v < 1000;
        requires len(v) <= 100;
        pragma aborts_if_is_partial;
    }

    // `spec_delta(a) == 0` is false once an iteration ran.
    fun loop_delta_false(v: &vector<u64>, a: address) {
        for_each_apply(v, a, |_y| { R[a].v = R[a].v + 1; }
            spec { ensures spec_delta(a) == 0; });
    }
    spec loop_delta_false {
        requires exists<R>(a) && R[a].v < 1000;
        requires len(v) <= 100;
        pragma aborts_if_is_partial;
    }
}
