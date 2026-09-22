module 0x42::runner {
    use std::vector;

    /// Left fold over a vector of u64 values.
    ///
    /// Applies `f` to each element in order, threading an accumulator:
    ///   acc_0     = init
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
        while (i < n) {
            acc = f(acc, *vector::borrow(v, i));
            i   = i + 1;
        };
        acc
    }
}
