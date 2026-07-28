// Reflected-over parameters are detected in ALL spec positions: inline spec
// blocks and observations hidden inside called spec function bodies also
// unpin the parameter. (With the parameter pinned, the observations would be
// provably false and the deliberately false assertions below would verify
// vacuously.)
module 0x42::partial_of_scan_fail {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        aborts_if s + e > MAX_U64;
        ensures s == old(s) + e;
    }

    /// Observation in an inline spec block only.
    fun observe_inline(f: |&u64| has copy + drop): u64 {
        spec {
            assume partial_of<f>(step);
            assert 1 == 2; // error: unknown assertion failed
        };
        0
    }

    spec fun is_step(f: |&u64| has copy + drop): bool {
        partial_of<f>(step)
    }

    /// Observation hidden inside a spec function body.
    fun observe_via(f: |&u64| has copy + drop): u64 {
        0
    }
    spec observe_via {
        requires is_step(f);
        ensures 1 == 2; // error: post-condition does not hold
    }

    inline fun call_once(f: |&u64| has copy + drop) {
        f(&1)
    }
    spec call_once {
        pragma opaque;
        ensures f == fun_post_of<old(f)>(1);
    }

    /// Makes the session's step variant exist.
    fun make_variant(): u64 {
        let s = 0;
        call_once(|e| step(&mut s, e));
        s
    }
}
