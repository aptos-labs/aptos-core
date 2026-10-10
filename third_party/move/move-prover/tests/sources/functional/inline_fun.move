module 0x42::Test {
    use std::vector;

    /// Removes all elements satisfying `predicate`, not preserving order. The
    /// loop invariant constrains the retained prefix via `result_of` over the
    /// predicate, which is derived per caller from the (spec-less) lambda.
    public inline fun filter<X: drop>(v: &mut vector<X>, predicate: |&X| bool) {
        let i = 0;
        while (i < vector::length(v)) {
            if (predicate(vector::borrow(v, i))) {
                vector::swap_remove(v, i);
            } else {
                i = i + 1;
            };
        } spec {
            invariant i <= len(v);
            invariant forall k in 0..i: !result_of<predicate>(v[k]);
        };
    }

    public fun test_filter(): vector<u64> {
        let v = vector[1u64, 2, 3];
        filter(&mut v, |e| *e > 1);
        v
    }
    spec test_filter {
        aborts_if false;
        ensures forall j in 0..len(result): result[j] <= 1;
    }
}
