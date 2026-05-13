module 0x42::fold_H {
    use std::vector;

    spec fun spec_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, i: u64): u64 { // [inferred]
        if (i == 0) { init } // [inferred]
        else { result_of<f>(spec_fold(f, v, init, i - 1), v[i - 1]) } // [inferred]
    } // [inferred]

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
                invariant i <= n; // [inferred]
                invariant acc == spec_fold(f, v, init, i); // [inferred]
                invariant forall k: u64 where k < i: !aborts_of<f>(spec_fold(f, v, init, k), v[k]); // [inferred]
            };
            i < n
        }) {
            acc = f(acc, *vector::borrow(v, i));
            i   = i + 1;
        };
        acc
    }
    spec fold(v: &vector<u64>, f: |u64, u64|u64 has copy + drop, init: u64): u64 {
        pragma opaque = true;
        aborts_if [inferred] exists x: u64: x < len(v) && aborts_of<f>(spec_fold(f, v, init, x), v[x]);
        ensures [inferred] (forall x: u64 where x < len(v): !aborts_of<f>(spec_fold(f, v, init, x), v[x])) ==> result == spec_fold(f, v, init, len(v));
    }

}
