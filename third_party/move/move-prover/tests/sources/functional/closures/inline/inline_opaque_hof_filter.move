// End-to-end test of `filter`: keep elements satisfying a predicate. Opaque
// spec characterizes the result by membership and predicate truth.
module 0x42::inline_opaque_hof_filter {
    use std::vector;

    inline fun filter<T: copy + drop>(v: &vector<T>, p: |&T| bool has copy + drop): vector<T> {
        let result = vector::empty<T>();
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            let e = vector::borrow(v, i);
            if (p(e)) {
                vector::push_back(&mut result, *e);
            };
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant forall j in 0..len(result):
                exists k in 0..i: result[j] == v[k] && result_of<p>(v[k]);
            invariant forall k in 0..i:
                result_of<p>(v[k]) ==> contains(result, v[k]);
            invariant forall j in 0..i: !aborts_of<p>(v[j]);
        };
        result
    }
    spec filter {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<p>(v[i]);
        aborts_if false;
        ensures forall j in 0..len(result):
            exists k in 0..len(v): result[j] == v[k] && result_of<p>(v[k]);
        ensures forall k in 0..len(v):
            result_of<p>(v[k]) ==> contains(result, v[k]);
    }

    /// Caller: keep only even elements. The result contains exactly the even
    /// inputs; non-even elements are absent. Caller proves both directions.
    fun keep_even(v: &vector<u64>): vector<u64> {
        filter(v, |e| (*e % 2 == 0) spec {
            aborts_if false;
            ensures result == (e % 2 == 0);
        })
    }
    spec keep_even {
        aborts_if false;
        ensures forall j in 0..len(result): result[j] % 2 == 0;
        ensures forall k in 0..len(v): v[k] % 2 == 0 ==> contains(result, v[k]);
    }

    /// Caller: drop zeros from a vector.
    fun drop_zeros(v: &vector<u64>): vector<u64> {
        filter(v, |e| (*e != 0) spec {
            aborts_if false;
            ensures result == (e != 0);
        })
    }
    spec drop_zeros {
        aborts_if false;
        ensures forall j in 0..len(result): result[j] != 0;
        ensures forall k in 0..len(v): v[k] != 0 ==> contains(result, v[k]);
    }
}
