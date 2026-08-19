// The general form `folds_of<f>(g, i)` (see `folds_of.move` for the
// element form): the literal index lambda `g` maps iteration `j` to the
// target's argument tuple — enumerations, zips, and reversed iteration
// orders. The inliner composes `g` into the derived accumulator
// transformer and specializes the module's `spec_fold_idx` recursion, so
// caller restatements of the composed `|acc, j|` transformer unify onto
// the same specialized symbol.
module 0x42::folds_of_idx {
    use std::vector;

    /// The index-fold recursion specialized by the general form of
    /// `folds_of`, resolved in the current module.
    spec fun spec_fold_idx<Acc>(t: |Acc, u64| Acc, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<t>(spec_fold_idx(t, init, end - 1), end - 1)
    }

    // ===== enumeration =====

    inline fun enumerate_ref<T>(v: &vector<T>, f: |u64, &T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(i, vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(|j| (j, v[j]), i);
        };
    }

    /// The user's abstraction: the sum of the elements plus the sum of
    /// their indices.
    spec fun spec_idx_sum(v: vector<u64>, n: num): num {
        if (n == 0) 0 else spec_idx_sum(v, n - 1) + (n - 1) + v[n - 1]
    }

    /// Bridging lemma: the composed transformer restated literally unifies
    /// with the one derived from `|i, e| sum = sum + i + *e` under
    /// `|j| (j, v[j])`.
    spec lemma idx_fold_is_idx_sum(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold_idx<u64>(|acc, j| acc + j + v[j], 0, n) == spec_idx_sum(v, n);
    } proof {
        if (n > 0) {
            apply idx_fold_is_idx_sum(v, n - 1);
        }
    }

    /// Exact postcondition for an enumerating accumulation, through the
    /// bridging lemma.
    fun idx_sum(v: &vector<u64>): u64 {
        let sum = 0;
        enumerate_ref(v, |i, e| sum = sum + i + *e);
        sum
    }
    spec idx_sum {
        pragma aborts_if_is_partial;
        ensures result == spec_idx_sum(v, len(v));
    } proof {
        apply idx_fold_is_idx_sum(v, len(v));
        // Resolve every fold term (also those of the prefix no-abort
        // invariant) equationally, keeping the solver out of definitional
        // unfolding chains.
        forall n: u64 {spec_fold_idx<u64>(|acc, j| acc + j + v[j], 0, n)}
            apply idx_fold_is_idx_sum(v, n);
    }

    /// A multi-capture accumulation through the general form: the pair
    /// accumulator folds with a generated tuple-returning index recursion
    /// (exact at concrete lengths; compare `folds_of_multi.move`).
    fun pair_enumerate(v: &vector<u64>): (u64, u64) {
        let sum = 0;
        let cnt = 0;
        enumerate_ref(v, |i, e| {
            sum = sum + i + *e;
            cnt = cnt + 1;
        });
        (sum, cnt)
    }
    spec pair_enumerate {
        pragma aborts_if_is_partial;
        ensures len(v) == 2 ==> result_1 == v[0] + v[1] + 1 && result_2 == 2;
    }

    // ===== zip =====

    inline fun zip_ref<T>(v1: &vector<T>, v2: &vector<T>, f: |&T, &T|) {
        let i = 0;
        let n = vector::length(v1);
        while (i < n) {
            f(vector::borrow(v1, i), vector::borrow(v2, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v1);
            invariant folds_of<f>(|j| (v1[j], v2[j]), i);
        };
    }

    /// Exact postcondition for a zipped accumulation, as a direct
    /// restatement of the composed transformer.
    fun pair_sum(a: &vector<u64>, b: &vector<u64>): u64 {
        let sum = 0;
        zip_ref(a, b, |x, y| sum = sum + *x + *y);
        sum
    }
    spec pair_sum {
        pragma aborts_if_is_partial;
        requires len(a) == len(b);
        ensures result == spec_fold_idx<u64>(|acc, j| acc + a[j] + b[j], 0, len(a));
    }

    // ===== reversed iteration order =====

    inline fun rev_each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, n - 1 - i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(|j| v[len(v) - 1 - j], i);
        };
    }

    /// Exact postcondition for a reversed accumulation, as a direct
    /// restatement of the composed transformer.
    fun rev_sum(v: &vector<u64>): u64 {
        let sum = 0;
        rev_each_ref(v, |e| sum = sum + *e);
        sum
    }
    spec rev_sum {
        pragma aborts_if_is_partial;
        ensures result == spec_fold_idx<u64>(|acc, j| acc + v[len(v) - 1 - j], 0, len(v));
    }

    /// Non-vacuity canary: a wrong equation must fail.
    fun rev_sum_wrong(v: &vector<u64>): u64 {
        let sum = 0;
        rev_each_ref(v, |e| sum = sum + *e);
        sum
    }
    spec rev_sum_wrong {
        pragma aborts_if_is_partial;
        // error: off by one
        ensures result == spec_fold_idx<u64>(|acc, j| acc + v[len(v) - 1 - j], 0, len(v)) + 1;
    }
}
