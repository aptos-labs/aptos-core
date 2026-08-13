// A fully symbolic sum over a vector via an inline `fold`, specified exactly
// against the user's own recursive `spec_sum` abstraction. The bridge between
// the two recursions is an inductive lemma whose `spec_fold` call restates
// the code's lambda literally: literal lambda arguments of spec function
// calls are specialized like lambda-bound ones, and spec-equivalent lambdas
// unify onto the same specialized function, so the lemma and the expanded
// loop invariant speak about the same symbol. Note the explicit type
// arguments `spec_fold<u64, u64>`: they pin the instantiation to the code
// side's, which spec-mode number widening would otherwise generalize to
// `num`.
module 0x42::fold_symbolic {
    use std::vector;

    inline fun fold<T, Acc: copy + drop>(
        v: &vector<T>,
        init: Acc,
        f: |Acc, &T| Acc has copy + drop,
    ): Acc {
        let acc = init;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            acc = f(acc, vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant acc == spec_fold(f, v, init, i);
            invariant forall j in 0..i: !aborts_of<f>(spec_fold(f, v, init, j), v[j]);
        };
        acc
    }

    /// Recursive definition of fold over the prefix `v[0..end]`.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// The user's own abstraction: sum of the first `n` elements.
    spec fun spec_sum(v: vector<u64>, n: num): num {
        if (n == 0) 0 else spec_sum(v, n - 1) + v[n - 1]
    }

    /// Bridging lemma: the fold with the addition lambda equals `spec_sum`.
    /// Proven by induction; both sides unfold one step.
    spec lemma fold_is_sum(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n) == spec_sum(v, n);
    } proof {
        if (n > 0) {
            apply fold_is_sum(v, n - 1);
        }
    }

    /// A partial sum plus the next element is bounded by any later partial
    /// sum (elements are non-negative). Proven by induction.
    spec lemma sum_step_bound(v: vector<u64>, i: u64, n: u64) {
        requires i < n && n <= len(v);
        ensures spec_sum(v, i) + v[i] <= spec_sum(v, n);
    } proof {
        if (i + 1 < n) {
            apply sum_step_bound(v, i, n - 1);
        }
    }

    /// The exact abort condition (in both directions: `sum_step_bound`
    /// lifts the overflowing step to the total, and at normal exit the
    /// u64-typed accumulator bounds the total) and the exact functional
    /// postcondition.
    fun sum(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + *e)
    }
    spec sum {
        aborts_if spec_sum(v, len(v)) > MAX_U64;
        ensures result == spec_sum(v, len(v));
    } proof {
        apply fold_is_sum(v, len(v));
        forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)}
            apply fold_is_sum(v, n);
        forall i: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, i)}
            apply sum_step_bound(v, i, len(v));
    }

    /// A symbolic caller specified directly in terms of `spec_fold` needs no
    /// lemma at all: the literal lambda unifies with the loop invariant's
    /// specialization, so the postcondition is the loop-exit fact verbatim.
    fun sum_direct(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + *e)
    }
    spec sum_direct {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
    }

    /// Non-vacuity canary: a wrong postcondition must fail.
    fun sum_wrong(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + *e)
    }
    spec sum_wrong {
        pragma aborts_if_is_partial;
        ensures result == spec_sum(v, len(v)) + 1; // error: off by one
    } proof {
        apply fold_is_sum(v, len(v));
    }
}
