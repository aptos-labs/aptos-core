module 0x42::collection {
    use std::vector;

    /// Find the first index `i < length(v)` such that `pred(&v[i])`
    /// holds; returns `length(v)` if no such index exists.
    public fun find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (pred(&v[i])) return i;
            i = i + 1;
        };
        n
    }


    /// Named predicate, given an explicit spec so callers of `find`
    /// can propagate its contract.
    public fun is_zero(x: &u64): bool {
        *x == 0
    }

    /// Zero-predicate specialisation of `find`.
    public fun find_zero(v: &vector<u64>): u64 {
        find(v, is_zero)
    }

}
