// End-to-end test of `fold`: functional left-fold producing an accumulator.
// The opaque spec is given by a recursive `spec fun spec_fold` that mirrors
// the body's iteration order. This is the canonical way to express
// "sum based on foreach" with an opaque HOF: the accumulator is threaded
// through the lambda's value-returning shape, avoiding `&mut` captures
// (which would be incompatible with the `copy` ability required to call the
// closure in a loop).
module 0x42::inline_opaque_hof_fold {
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
    spec fold {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<f>(spec_fold(f, v, init, i), v[i]);
        aborts_if false;
        ensures result == spec_fold(f, v, init, len(v));
    }

    /// Recursive definition of fold over the prefix `v[0..end]`.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// Caller: sum of a concrete vector via fold. Demonstrates "sum based on
    /// foreach" — the accumulator is threaded through the lambda's return
    /// value rather than captured by `&mut`.
    fun sum_concrete(): u64 {
        let v = vector[1u64, 2, 3];
        fold(&v, 0, |acc, e| acc + *e spec {
            aborts_if acc + e > MAX_U64;
            ensures result == acc + e;
        })
    }
    spec sum_concrete {
        ensures result == 6;
    }

    /// Caller: product of a concrete vector via fold.
    fun product_concrete(): u64 {
        let v = vector[2u64, 3, 4];
        fold(&v, 1, |acc, e| acc * *e spec {
            aborts_if acc * e > MAX_U64;
            ensures result == acc * e;
        })
    }
    spec product_concrete {
        ensures result == 24;
    }

    /// Caller: count even elements in a concrete vector via fold. The lambda
    /// uses control flow.
    fun count_even_concrete(): u64 {
        let v = vector[1u64, 2, 3, 4, 5, 6];
        fold(&v, 0, |acc, e| acc + (if (*e % 2 == 0) 1 else 0) spec {
            aborts_if acc + (if (e % 2 == 0) 1 else 0) > MAX_U64;
            ensures result == acc + (if (e % 2 == 0) 1 else 0);
        })
    }
    spec count_even_concrete {
        ensures result == 3;
    }

    /// Caller: fold over the empty vector — the accumulator is unchanged.
    fun fold_empty(): u64 {
        let v = vector::empty<u64>();
        fold(&v, 42, |acc, e| acc + *e spec {
            aborts_if acc + e > MAX_U64;
            ensures result == acc + e;
        })
    }
    spec fold_empty {
        ensures result == 42;
    }

    /// Caller: max element of a non-empty vector via fold seeded with 0.
    /// Demonstrates a value-capturing accumulator update. The explicit
    /// `acc: u64` annotation disambiguates `>` (Move 2 treats `>` as either
    /// `(&T, &T): bool` or `(T, T): bool`).
    fun max_three(): u64 {
        let v = vector[3u64, 7, 2];
        fold(&v, 0u64, |acc: u64, e| if (*e > acc) *e else acc spec {
            aborts_if false;
            ensures result == (if (e > acc) e else acc);
        })
    }
    spec max_three {
        ensures result == 7;
    }
}
