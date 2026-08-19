// The collection pattern through `folds_of` (see `folds_of.move` for the
// single-capture core): the capture is the collected vector, the derived
// transformer appends, and the bridging lemma identifies the fold with the
// input prefix.
module 0x42::folds_of_collect {
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

    /// The fold recursion specialized by `folds_of` for single-slot
    /// accumulators, resolved in the current module.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }


    /// The collection pattern (the former `ensures_of` capture-write
    /// canary, now a positive): the capture is the collected vector, the
    /// transformer appends, and the bridging lemma identifies the fold
    /// with the input prefix.
    fun collect(v: &vector<u64>): vector<u64> {
        let r = vector[];
        each_ref(v, |e| vector::push_back(&mut r, *e));
        r
    }
    spec collect {
        aborts_if false;
        ensures result == v;
    } proof {
        apply fold_is_prefix(v, len(v));
    }

    /// Bridging lemma: appending the first `n` elements yields the prefix
    /// `v[0..n]`.
    spec lemma fold_is_prefix(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, vector<u64>>(|acc, e| concat(acc, vec(e)), v, vec(), n)
            == v[0..n];
    } proof {
        if (n > 0) {
            apply fold_is_prefix(v, n - 1);
        }
    }

    /// Non-vacuity canary for the collection equation.
    fun collect_wrong(v: &vector<u64>): vector<u64> {
        let r = vector[];
        each_ref(v, |e| vector::push_back(&mut r, *e));
        r
    }
    spec collect_wrong {
        pragma aborts_if_is_partial;
        ensures len(result) == len(v) + 1; // error: collected exactly the input
    } proof {
        apply fold_is_prefix(v, len(v));
    }
}
