// Tests that a `&mut`-capturing lambda is admitted for a function parameter
// requiring the `copy` ability. The ability BOUND is admitted (loop-invoking
// HOF signatures require it); the closure value itself remains linear at the
// model level (a surviving copy is rejected). Callee-side coherence of
// `ensures_of` is anchored by the `result_of` successor, so a spec claiming
// one application is provable exactly when the body applies the parameter
// once (see opaque_inline_apply_discipline_fail for the rejected bodies).
module 0x42::opaque_inline_mut_capture_copy {

    inline fun call_once_copy(f: |u64| has copy + drop) {
        f(2)
    }
    spec call_once_copy {
        pragma opaque;
        ensures ensures_of<f>(2);
    }

    /// Test: a mutating lambda satisfies a `copy` bound; the single
    /// application fact propagates to the caller.
    fun test_copy_bound(): u64 {
        let x = 0;
        call_once_copy(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_copy_bound {
        ensures result == 2;
    }

    inline fun call_twice_weak(f: |u64| has copy + drop) {
        f(1);
        f(1);
    }
    spec call_twice_weak {
        pragma opaque;
        // No `ensures_of`: a twice-applying body cannot describe the
        // accumulated effect with a single `ensures_of` claim, and chained
        // claims need `fun_post_of` (issue #20273).
    }

    /// Test: with a weak callee spec the capture is havoced and unconstrained
    /// at the call site — the caller cannot conclude a post value (but the
    /// havoc is sound: no false facts either).
    fun test_weak_spec(): u64 {
        let x = 0;
        call_twice_weak(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_weak_spec {
        ensures result == 2; // error: post-condition does not hold
    }
}
