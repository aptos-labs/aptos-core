// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A fold over two captures generates a specification function returning both,
// which the loop invariant binds with a tuple `let` under a quantifier.
module 0x42::fold_captures {
    inline fun each<E>(v: &vector<E>, f: |&E|) {
        let i = 0;
        let len = std::vector::length(v);
        while (i < len) {
            f(std::vector::borrow(v, i));
            i += 1
        } spec {
            invariant i <= len;
            invariant forall j in 0..i: ensures_of<f>(v[j]);
            invariant folds_of<f>(v, i);
        };
    }

    fun count(v: &vector<u64>): (u64, u64) {
        let small = 0;
        let large = 0;
        each(v, |x| {
            if (*x < 10) { small += 1 } else { large += 1 };
        });
        (small, large)
    }
    spec count {
        pragma aborts_if_is_partial;
        ensures result_1 == result_1;
    }

    fun count_wrong(v: &vector<u64>): (u64, u64) {
        let small = 0;
        let large = 0;
        each(v, |x| {
            if (*x < 10) { small += 1 } else { large += 1 };
        });
        (small, large)
    }
    spec count_wrong {
        pragma aborts_if_is_partial;
        ensures result_1 == 1; // error: nothing bounds the count from below
    }
}
