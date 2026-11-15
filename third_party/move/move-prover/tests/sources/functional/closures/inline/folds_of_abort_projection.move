// Per-capture projection of a multi-capture `folds_of`: a captured local
// whose update reads only itself and the element folds on its own, through
// the same `spec_fold` a single capture uses; a local whose update reads
// another capture stays in that capture's group.
module 0x42::folds_of_abort_projection {
    use std::vector;

    /// The generic HOF, as in `folds_of.move`.
    inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(v, i);
        };
    }

    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    // The dependent capture sorts first, so its tuple group precedes b's
    // singleton group. Abort contracts must still use b's named spec_fold.
    fun dependent_first_abort(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |_e| {
            assert!(b < 100, 1);
            a = b;
            b = b + 1;
        });
        (a, b)
    }
    spec dependent_first_abort {
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, _e| acc + 1, v, 0, j) >= 100;
        ensures result_2 == spec_fold<u64, u64>(|acc, _e| acc + 1, v, 0, len(v));
    }

    // Non-vacuity: a different abort threshold must not verify.
    fun dependent_first_abort_wrong(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |_e| {
            assert!(b < 100, 1);
            a = b;
            b = b + 1;
        });
        (a, b)
    }
    spec dependent_first_abort_wrong {
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, _e| acc + 1, v, 0, j) >= 101; // error: aborts at 100
    }

}
