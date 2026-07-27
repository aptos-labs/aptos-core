// Regression: a fun param reflected over only through a NESTED observation
// argument (`partial_of<fun_post_of<f>(1)>(step)`) is still verified
// universally. With the param pinned to its abstract variant, the premise
// would be provably false and the deliberately false conclusion below would
// verify vacuously — while call sites CAN discharge the premise (the
// successor of a concrete step-closure stays in the step variant).
module 0x42::partial_of_nested_fail {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
    }

    inline fun bump_nested(f: |&u64| has copy + drop) {
        f(&1)
    }
    spec bump_nested {
        pragma opaque;
        requires partial_of<fun_post_of<f>(1)>(step);
        requires captures_of<fun_post_of<f>(1)>(step) < 1000;
        ensures 1 == 2; // error: post-condition does not hold
    }

    /// Makes the session's step variant exist; the call site can discharge
    /// the nested premise at the concrete closure.
    fun make_variant(): u64 {
        let s = 0;
        bump_nested(|e| step(&mut s, e));
        s
    }
}
