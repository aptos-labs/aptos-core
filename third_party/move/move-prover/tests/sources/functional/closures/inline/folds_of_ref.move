// The through-reference form of `folds_of` (see `folds_of.move` for the
// single-capture core): a coupled accumulation behind a captured `&mut`
// reference folds the referenced struct value with the generic
// (restatable) `spec_fold`, so symbolic-length facts flow through a
// bridging lemma.
module 0x42::folds_of_ref {
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


    struct Acc has copy, drop {
        total: u64,
        count: u64,
    }

    /// Coupled accumulation through a captured `&mut` reference: the
    /// accumulator is the referenced struct value, folded by the generic
    /// `spec_fold` — so the recursion is restatable and symbolic-length
    /// facts flow through a bridging lemma (this is the supported route
    /// for coupled state, compare `weighted_pair` above).
    fun weighted_via_ref(v: &vector<u64>): (u64, u64) {
        let acc = Acc { total: 0, count: 0 };
        let r = &mut acc;
        each_ref(v, |e| {
            r.total = r.total + r.count + *e;
            r.count = r.count + 1;
        });
        (acc.total, acc.count)
    }
    spec weighted_via_ref {
        pragma aborts_if_is_partial;
        ensures result_1 == spec_idx_sum(v, len(v));
        ensures result_2 == len(v);
    } proof {
        apply ref_fold_is_weighted(v, len(v));
        // Resolve every fold term (also those of the prefix no-abort
        // invariant) equationally, keeping the solver out of definitional
        // unfolding chains.
        forall n: u64 {spec_fold<u64, Acc>(
            |acc: Acc, e| update_field(update_field(acc, total, acc.total + acc.count + e),
                                       count, acc.count + 1),
            v, Acc { total: 0, count: 0 }, n)}
            apply ref_fold_is_weighted(v, n);
    }

    /// The user's abstraction of the coupled accumulation: the sum of the
    /// elements plus the sum of their indices.
    spec fun spec_idx_sum(v: vector<u64>, n: num): num {
        if (n == 0) 0 else spec_idx_sum(v, n - 1) + (n - 1) + v[n - 1]
    }

    /// Bridging lemma for the through-reference fold: the restated
    /// transformer (a functional double field update over the struct
    /// accumulator) unifies with the one derived from the lambda.
    spec lemma ref_fold_is_weighted(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, Acc>(
            |acc: Acc, e| update_field(update_field(acc, total, acc.total + acc.count + e),
                                       count, acc.count + 1),
            v, Acc { total: 0, count: 0 }, n)
            == Acc { total: spec_idx_sum(v, n), count: n };
    } proof {
        if (n > 0) {
            apply ref_fold_is_weighted(v, n - 1);
        }
    }

    /// Non-vacuity canary for the through-reference equation.
    fun weighted_via_ref_wrong(v: &vector<u64>): u64 {
        let acc = Acc { total: 0, count: 0 };
        let r = &mut acc;
        each_ref(v, |e| {
            r.total = r.total + r.count + *e;
            r.count = r.count + 1;
        });
        acc.count
    }
    spec weighted_via_ref_wrong {
        pragma aborts_if_is_partial;
        ensures len(v) == 1 ==> result == 2; // error: the count of a singleton is 1
    }
}
