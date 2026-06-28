// End-to-end test of `map`: build a new vector by applying a pure function to
// each element. Opaque spec characterizes length and pointwise relation via
// `result_of<f>`.
module 0x42::inline_opaque_hof_map {
    use std::vector;

    inline fun map<T, U>(v: &vector<T>, f: |&T| U has copy + drop): vector<U> {
        let result = vector::empty<U>();
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            vector::push_back(&mut result, f(vector::borrow(v, i)));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant len(result) == i;
            invariant forall j in 0..i: result[j] == result_of<f>(v[j]);
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
        };
        result
    }
    spec map {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == result_of<f>(v[i]);
    }

    /// Caller: doubles each element.
    fun doubles(v: &vector<u64>): vector<u64> {
        map(v, |e| *e * 2 spec {
            aborts_if e * 2 > MAX_U64;
            ensures result == e * 2;
        })
    }
    spec doubles {
        requires forall i in 0..len(v): v[i] * 2 <= MAX_U64;
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] * 2;
    }

    /// Caller: increments by a captured constant.
    fun bump_by(v: &vector<u64>, c: u64): vector<u64> {
        map(v, |e| *e + c spec {
            aborts_if e + c > MAX_U64;
            ensures result == e + c;
        })
    }
    spec bump_by {
        requires forall i in 0..len(v): v[i] + c <= MAX_U64;
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] + c;
    }

    /// Caller: lambda returns a bool. Demonstrates a type-changing `map`.
    fun mark_positive(v: &vector<u64>): vector<bool> {
        map(v, |e| (*e > 0) spec {
            aborts_if false;
            ensures result == (e > 0);
        })
    }
    spec mark_positive {
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == (v[i] > 0);
    }
}
