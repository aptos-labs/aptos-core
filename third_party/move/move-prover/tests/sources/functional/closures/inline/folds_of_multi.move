// The multi-capture form of `folds_of` (see `folds_of.move` for the
// single-capture core): coupled updates of two captured locals fold with a
// generated tuple-returning recursion.
module 0x42::folds_of_multi {
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

    // ===== multi-capture tuples =====

    /// Coupled updates of two captures: the accumulator is the pair
    /// `(count, sum)`, folded by a generated tuple-returning recursion (no
    /// generic `spec_fold` instance exists — tuples are not expressible as
    /// spec type arguments, which also makes the recursion unnameable in
    /// surface specs; symbolic-length facts for multi-local captures are
    /// therefore out of reach — couple the state in a struct behind a
    /// `&mut` capture instead, see `weighted_via_ref` below). The equation
    /// is exact, as the concrete-length unfoldings show.
    fun weighted_pair(v: &vector<u64>): (u64, u64) {
        let sum = 0;
        let count = 0;
        each_ref(v, |e| {
            sum = sum + count + *e;
            count = count + 1;
        });
        (sum, count)
    }
    spec weighted_pair {
        pragma aborts_if_is_partial;
        ensures len(v) == 0 ==> result_1 == 0 && result_2 == 0;
        ensures len(v) == 2 ==> result_1 == v[0] + v[1] + 1 && result_2 == 2;
    }

    /// Non-vacuity canary for the pair equation.
    fun weighted_pair_wrong(v: &vector<u64>): (u64, u64) {
        let sum = 0;
        let count = 0;
        each_ref(v, |e| {
            sum = sum + count + *e;
            count = count + 1;
        });
        (sum, count)
    }
    spec weighted_pair_wrong {
        pragma aborts_if_is_partial;
        ensures len(v) == 2 ==> result_1 == v[0] + v[1]; // error: the count weight adds one
    } proof {
        post {
            if (len(v) == 2) {
                assert result_1 == v[0] + v[1] + 1;
            }
        }
    }
}
