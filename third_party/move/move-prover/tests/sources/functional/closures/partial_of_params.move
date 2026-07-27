// Fun params observed by `partial_of`/`captures_of` are verified universally
// (not pinned to their abstract `$param` variant): the premise is satisfiable
// exactly for the matching closures, so conclusions must genuinely follow
// from it (see the _fail twin for non-vacuity), while call sites discharge
// the premise at concrete values.
module 0x42::partial_of_params {

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
        ensures f == fun_post_of<old(f)>(1);
    }

    /// Test: the premise is discharged at the concrete closure, and the
    /// captured accumulator advances by one.
    fun use_bump(): u64 {
        let s = 7;
        bump_once(|e| step(&mut s, e));
        s
    }
    spec use_bump {
        ensures result == 8;
    }
}
