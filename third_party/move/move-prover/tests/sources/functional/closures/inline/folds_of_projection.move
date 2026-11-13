// Per-capture projection of a multi-capture `folds_of`: a captured local
// whose update reads only itself and the element folds on its own, through
// the same `spec_fold` a single capture uses; a local whose update reads
// another capture stays in that capture's group.
module 0x42::folds_of_projection {
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

    // ===== independent captures: each projects =====

    /// Two totals, each reading only itself and the element.
    fun independent(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + 1;
        });
        (a, b)
    }
    spec independent {
        pragma aborts_if_is_partial;
        ensures result_1 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, len(v));
    }

    /// Non-vacuity: a wrong transformer for one projected capture.
    fun independent_wrong(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + 1;
        });
        (a, b)
    }
    spec independent_wrong {
        pragma aborts_if_is_partial;
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc + 2, v, 0, len(v)); // error: b counts by one
    }

    /// A filtered total next to a total: the shape of a sum and a spendable sum.
    fun filtered(v: &vector<u64>): (u64, u64) {
        let total = 0;
        let small = 0;
        each_ref(v, |e| {
            total = total + *e;
            if (*e < 10) { small = small + *e; };
        });
        (total, small)
    }
    spec filtered {
        pragma aborts_if_is_partial;
        ensures result_1 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
        ensures result_2 == spec_fold<u64, u64>(|acc, e| if (e < 10) acc + e else acc, v, 0, len(v));
    }

    // ===== dependent captures: must not project =====

    /// `b` reads `a` after `a`'s update: `b` is in `a`'s group.
    fun reads_updated(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + a;
        });
        (a, b)
    }
    spec reads_updated {
        pragma aborts_if_is_partial;
        // `a` reads only itself: stated alone it holds.
        ensures result_1 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
    }

    /// The same, with `b` claimed as an independent fold.
    fun reads_updated_wrong(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + a;
        });
        (a, b)
    }
    spec reads_updated_wrong {
        pragma aborts_if_is_partial;
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v)); // error: b adds a, not e
    }

    /// `b` reads `a`'s value from before the update, through a temporary.
    fun reads_prior_wrong(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            let prior = a;
            a = a + *e;
            b = b + prior;
        });
        (a, b)
    }
    spec reads_prior_wrong {
        pragma aborts_if_is_partial;
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc, v, 0, len(v)); // error: b adds the prior a
    }

    /// `b`'s update is guarded by a condition on `a`.
    fun guarded_wrong(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            if (a > 10) { b = b + 1; };
        });
        (a, b)
    }
    spec guarded_wrong {
        pragma aborts_if_is_partial;
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, len(v)); // error: b counts only once a passes 10
    }

    /// A chain: `c` reads `b`, which reads `a`. `c` alone is not a fold.
    fun chain_wrong(v: &vector<u64>): (u64, u64, u64) {
        let a = 0;
        let b = 0;
        let c = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + a;
            c = c + b;
        });
        (a, b, c)
    }
    spec chain_wrong {
        pragma aborts_if_is_partial;
        ensures result_3 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v)); // error: c adds b
    }

    // ===== a capture reading global state =====

    struct Config has key { scale: u64 }

    /// `b`'s update reads global state, so `b` has no equation; `a` is pure
    /// and keeps its own.
    fun mixed_state(v: &vector<u64>): (u64, u64) acquires Config {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + borrow_global<Config>(@0x42).scale;
        });
        (a, b)
    }
    spec mixed_state {
        pragma aborts_if_is_partial;
        ensures result_1 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
    }

    /// Non-vacuity: the dropped capture is not given a fold.
    fun mixed_state_wrong(v: &vector<u64>): (u64, u64) acquires Config {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            a = a + *e;
            b = b + borrow_global<Config>(@0x42).scale;
        });
        (a, b)
    }
    spec mixed_state_wrong {
        pragma aborts_if_is_partial;
        ensures result_2 == 0; // error: b accumulates the scale
    }

    // ===== aborts across groups =====

    /// The abort reads both captures, which project to separate folds: the
    /// condition must read each from its own fold at the same index.
    fun cross_abort(v: &vector<u64>): (u64, u64) {
        let a = 0;
        let b = 0;
        each_ref(v, |e| {
            assert!(a <= b + 1000, 1);
            a = a + *e;
            b = b + 1;
        });
        (a, b)
    }
    spec cross_abort {
        // The check's own arithmetic, then the check.
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, j) + 1000 > MAX_U64;
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, e| acc + e, v, 0, j)
                > spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, j) + 1000;
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, e| acc + e, v, 0, j) + v[j] > MAX_U64;
        aborts_if exists j in 0..len(v):
            spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, j) + 1 > MAX_U64;
        ensures result_1 == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
        ensures result_2 == spec_fold<u64, u64>(|acc, e| acc + 1, v, 0, len(v));
    }

    // Every capture depends transitively on global state. The assertion
    // also reads a dropped capture: the entire invariant must be weakened.
    fun transitively_impure(v: &vector<u64>): (u64, u64) acquires Config {
        let a = 0;
        let b = 0;
        each_ref(v, |_e| {
            let prior = a;
            a = a + borrow_global<Config>(@0x42).scale;
            b = b + prior;
            assert!(a < 100, 1);
        });
        (a, b)
    }
    spec transitively_impure {
        pragma aborts_if_is_partial;
        ensures result_2 >= 0;
    }

}
