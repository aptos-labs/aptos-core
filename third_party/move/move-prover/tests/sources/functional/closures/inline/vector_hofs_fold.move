// Verification of an inline higher-order `fold` similar to the one in the
// vector module, applied with different kinds of lambdas, including lambdas
// which access values of the enclosing context. Calls are expanded and
// lambdas beta-reduced; the loop invariant characterizes the accumulator
// with a recursive spec function over the function parameter (`spec_fold`),
// which is specialized per caller. See `vector_hofs_for_each.move` for the
// element-wise counterparts (map, find, for_each_ref, for_each_mut).
module 0x42::vector_hofs_fold {
    use std::vector;

    // ===== fold: the invariant characterizes the accumulator with a  =====
    // ===== recursive spec function over `f`, specialized per caller  =====

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

    /// Sum of a concrete vector via fold — "sum based on foreach", with the
    /// accumulator threaded through the lambda's return value.
    fun sum_concrete(): u64 {
        let v = vector[1u64, 2, 3];
        fold(&v, 0, |acc, e| acc + *e spec {
            aborts_if acc + e > MAX_U64;
            ensures result == acc + e;
        })
    }
    spec sum_concrete {
        aborts_if false;
        ensures result == 6;
    }

    /// The same as `sum_concrete`, with the lambda's spec inferred from its
    /// body: `result_of` is derived by beta reduction, and `aborts_of` from
    /// the abort conditions of the body's operations.
    fun sum_inferred(): u64 {
        let v = vector[1u64, 2, 3];
        fold(&v, 0, |acc, e| acc + *e)
    }
    spec sum_inferred {
        aborts_if false;
        ensures result == 6;
    }

    /// The accumulating lambda also reads `k` from the context.
    fun sum_scaled(k: u64): u64 {
        let v = vector[1u64, 2, 3];
        fold(&v, 0, |acc, e| acc + *e * k spec {
            aborts_if acc + e * k > MAX_U64;
            ensures result == acc + e * k;
        })
    }
    spec sum_scaled {
        requires k <= 1000;
        ensures result == 6 * k;
    }

    /// Product via fold.
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

    /// Count of even elements via fold; the lambda uses control flow.
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

    /// Fold over the empty vector — the accumulator is unchanged.
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

    /// Max element via fold seeded with 0. The explicit `acc: u64` annotation
    /// disambiguates `>` (Move 2 treats `>` as either `(&T, &T): bool` or
    /// `(T, T): bool`).
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
