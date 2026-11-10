// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// What `apply f.unfold(a)` rejects: an undecided guard, a function that does
// not recurse, and a compiler specialization, whose name no source states.
module 0x42::proof_unfold_errors {
    struct Item has drop { name: u64 }

    spec module {
        fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
            if (end == 0) init
            else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
        }
    }

    spec fun sum_upto(n: u64): num {
        if (n == 0) 0 else sum_upto(n - 1) + n
    }

    spec fun twice(n: u64): num { n + n }

    fun undecided(n: u64) { let _ = n; }
    spec undecided {
        aborts_if false;
        ensures sum_upto(n) >= n;
    }

    fun not_recursive(n: u64) { let _ = n; }
    spec not_recursive {
        aborts_if false;
        ensures twice(n) >= n;
    }

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

    fun names(v: &vector<Item>): vector<u64> {
        let out = std::vector::empty();
        each(v, |x| {
            let x: &Item = x;
            std::vector::push_back(&mut out, x.name);
        });
        out
    }
    spec names {
        pragma aborts_if_is_partial;
        ensures len(result) == len(v);
    }
}
