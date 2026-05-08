module 0x42::runner {
    use std::vector;

    // [inferred] Recursive helper: accumulator after applying f to the first k elements of v
    spec fun spec_fold(v: vector<u64>, f: |u64,u64|u64, acc: u64, k: u64): u64 {
        if (k == 0) { acc }
        else { result_of<f>(spec_fold(v, f, acc, k - 1), v[k - 1]) }
    }

    /// Left fold over a vector of u64 values.
    ///
    /// Applies `f` to each element in order, threading an accumulator:
    ///   acc_0  = init
    ///   acc_{i+1} = f(acc_i, v[i])
    ///
    /// `f` takes the current accumulator as its first argument and the
    /// current element as its second; it may abort depending on either.
    /// The fold aborts if any application of `f` aborts.
    public fun fold(
        v:    &vector<u64>,
        f:    |u64, u64| u64 has copy + drop,
        init: u64
    ): u64 {
        let acc = init;
        let i   = 0;
        let n   = vector::length(v);
        while ({
            spec {
                // [inferred]
                invariant i <= n;
                invariant n == vector::length(v);
                invariant acc == spec_fold(v, f, init, i);
                invariant forall j: u64 where j < i: !aborts_of<f>(spec_fold(v, f, init, j), v[j]);
            };
            i < n
        }) {
            acc = f(acc, *vector::borrow(v, i));
            i   = i + 1;
        };
        acc
    }

    spec fold {
        // [inferred] C1: fold aborts iff some application of f aborts
        aborts_if exists k: u64 where k < vector::length(v):
            aborts_of<f>(spec_fold(v, f, init, k), v[k]);
        // [inferred] C2: result is the complete fold
        ensures result == spec_fold(v, f, init, vector::length(v));
    }
}
