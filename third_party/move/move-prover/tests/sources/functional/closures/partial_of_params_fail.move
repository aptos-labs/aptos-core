// Non-vacuity of `partial_of` premises: the subject param is verified
// universally, so a false conclusion under a satisfiable premise must FAIL
// (with the param pinned to its abstract variant, the premise would be
// provably false and this would verify vacuously).
module 0x42::partial_of_params_fail {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
    }

    inline fun bump_once(f: |&u64| has copy + drop) {
        f(&1)
    }
    spec bump_once {
        pragma opaque;
        requires partial_of<f>(step);
        requires captures_of<f>(step) < 1000;
        ensures captures_of<fun_post_of<old(f)>(1)>(step)
             == captures_of<old(f)>(step) + 2; // error: post-condition does not hold
    }

    /// Makes the session's step variant exist (the premise is satisfiable).
    fun make_variant(): u64 {
        let s = 0;
        bump_once(|e| step(&mut s, e));
        s
    }
}
