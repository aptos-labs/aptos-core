// Projected consuming folds must number anchored context reads per group.
module 0x42::folds_of_projection_anchors {
    use std::vector;

    spec fun spec_fold_idx<Acc>(f: |Acc, u64| Acc, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold_idx(f, init, end - 1), end - 1)
    }

    inline fun each_pair(v1: vector<u64>, v2: vector<u64>, f: |u64, u64|) {
        let n = vector::length(&v1);
        assert!(n == vector::length(&v2), 1);
        let remaining = n;
        while (remaining > 0) {
            f(vector::pop_back(&mut v1), vector::pop_back(&mut v2));
            remaining = remaining - 1;
        } spec {
            invariant remaining <= n;
            invariant remaining == len(v1) && remaining == len(v2);
            invariant n == len(old(v1)) && n == len(old(v2));
            invariant forall j in 0..remaining: v1[j] == old(v1)[j];
            invariant forall j in 0..remaining: v2[j] == old(v2)[j];
            invariant folds_of<f>(|j| (old(v1)[n - 1 - j], old(v2)[n - 1 - j]), n - remaining);
        };
        vector::destroy_empty(v1);
        vector::destroy_empty(v2);
    }

    // The second projection uses the second anchored vector. A standalone
    // capture with the same update must reuse its recursion.
    fun both(): (u64, u64) {
        let a = 0;
        let b = 0;
        each_pair(vector[1, 2, 3], vector[4, 5, 6], |x, y| {
            a = a + x;
            b = b + y;
        });
        (a, b)
    }
    spec both {
        aborts_if false;
        ensures result_1 == 6;
        ensures result_2 == 15;
    }

    fun second(): u64 {
        let b = 0;
        each_pair(vector[1, 2, 3], vector[4, 5, 6], |_x, y| b = b + y);
        b
    }
    spec second {
        aborts_if false;
        ensures result == 15;
    }

    fun wrong(): u64 {
        let a = 0;
        let b = 0;
        each_pair(vector[1, 2, 3], vector[4, 5, 6], |x, y| {
            a = a + x;
            b = b + y;
        });
        b
    }
    spec wrong {
        aborts_if false;
        ensures result == 6; // error: the second vector sums to 15
    }
}
