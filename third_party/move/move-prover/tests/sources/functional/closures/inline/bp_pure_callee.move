// Lambda callees resolved through companion spec functions and memory-free
// callee summaries. A lambda body calling a *pure* named function resolves
// the call as a direct spec-function call (companions are derived before
// inlining), and a call to a memory-free (abort-only) function keeps the
// summary single-state with an exact (empty) modifies footprint — so
// `folds_of` and `unchanged_of` stay derivable. The vector intrinsics
// `index_of`/`swap_remove`/`append` have exact WPs and remain derivable in
// capture-writing lambdas as well.
module 0x42::bp_pure_callee {
    use std::vector;

    /// The generic HOF with the full invariant set: `folds_of` plus the
    /// `unchanged_of` frame, which requires the lambda's exact memory
    /// footprint.
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
            invariant forall j in i..n: unchanged_of<f>(v[j]);
        };
    }

    /// The fold recursion specialized by `folds_of` (restatable by
    /// callers); resolved in the current module.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// A pure error-code constructor, as in `std::error`.
    fun err(r: u64): u64 {
        0x10000 + r
    }

    /// The validator pattern: the abort code expression calls a pure named
    /// function. The derived abort condition stays single-state through the
    /// callee's companion spec function, so `folds_of` (degenerating to the
    /// prefix no-abort condition) and the empty-footprint `unchanged_of`
    /// resolve, and the exact abort condition verifies.
    fun check_all_nonzero(v: &vector<u64>) {
        each_ref(v, |e| assert!(*e != 0, err(1)));
    }
    spec check_all_nonzero {
        aborts_if exists i in 0..len(v): v[i] == 0;
    }

    /// Accumulation combined with a pure-callee validator: the capture
    /// transformer and abort disjuncts stay pure and single-state, so the
    /// caller restatement against `spec_fold` verifies without a lemma.
    fun sum_checked(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| {
            assert!(*e != 0, err(1));
            sum = sum + *e;
        });
        sum
    }
    spec sum_checked {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
    }

    /// An abort-only validator: no memory effects, so the behavioral
    /// summary of the call is single-state and the exact (empty) modifies
    /// footprint is kept.
    fun check_small(x: u64) {
        assert!(x < 1000, 1);
    }

    /// The counting fold equals the iteration count.
    spec lemma count_fold_is_len(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, n) == n;
    } proof {
        if (n > 0) {
            apply count_fold_is_len(v, n - 1);
        }
    }

    /// Accumulation through a lambda which also calls an abort-only
    /// function: `folds_of` and `unchanged_of` resolve (the callee summary
    /// is single-state with an empty footprint), and the count is exact.
    fun count_all(v: &vector<u64>): u64 {
        let count = 0;
        each_ref(v, |e| {
            check_small(*e);
            count = count + 1;
        });
        count
    }
    spec count_all {
        pragma aborts_if_is_partial;
        ensures result == len(v);
    } proof {
        apply count_fold_is_len(v, len(v));
        forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, n)}
            apply count_fold_is_len(v, n);
    }

    /// The owner-removal shape: `index_of` and `swap_remove` in a
    /// capture-writing lambda, derivable through their exact WPs. The
    /// `folds_of` equation and the prefix no-abort condition (in-bounds
    /// `swap_remove` guarded by `index_of`'s found flag) verify at the
    /// expansion.
    fun remove_all_found(keys: vector<u64>, remove: &vector<u64>): vector<u64> {
        let owners = keys;
        each_ref(remove, |r| {
            let (found, idx) = vector::index_of(&owners, r);
            if (found) {
                vector::swap_remove(&mut owners, idx);
            }
        });
        owners
    }
    spec remove_all_found {
        aborts_if false;
    }

    /// `append` in a capture-writing lambda, via its exact WP.
    fun flatten(vs: &vector<vector<u64>>): vector<u64> {
        let acc = vector[];
        each_ref(vs, |v| vector::append(&mut acc, *v));
        acc
    }
    spec flatten {
        aborts_if false;
    }
}
