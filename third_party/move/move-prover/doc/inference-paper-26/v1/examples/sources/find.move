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
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] n == len(v);
            invariant [inferred] forall j: u64 where j < i: !result_of<pred>(v[j]);
            invariant [inferred] forall j: u64 where j < i: !aborts_of<pred>(v[j]);
        };
        n
    }
    spec find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        pragma opaque = true;
        ensures [inferred] result <= len(v);
        ensures [inferred] forall j: u64 where j < result: !result_of<pred>(v[j]);
        ensures [inferred] result < len(v) ==> result_of<pred>(v[result]);
        aborts_if [inferred] exists j: u64 where j < len(v):
            aborts_of<pred>(v[j])
            && (forall k: u64 where k < j: !result_of<pred>(v[k]) && !aborts_of<pred>(v[k]));
    }




    /// Named predicate, given an explicit spec so callers of `find`
    /// can propagate its contract.
    public fun is_zero(x: &u64): bool {
        *x == 0
    }
    spec is_zero(x: &u64): bool {
        pragma opaque = true;
        ensures [inferred] result == (x == 0);
        aborts_if [inferred] false;
    }


    /// Zero-predicate specialisation of `find`.
    public fun find_zero(v: &vector<u64>): u64 {
        find(v, is_zero)
    }
    spec find_zero(v: &vector<u64>): u64 {
        pragma opaque = true;
        ensures [inferred] result == result_of<find<u64>>(v, |x| is_zero(x));
        aborts_if [inferred] aborts_of<find<u64>>(v, |x| is_zero(x));
    }

}
