// End-to-end test of an inline higher-order function `for_each_ref` with an
// opaque spec. The body verifies standalone via a forward-index loop invariant
// and the spec uses behavioral predicates over each element. Callers reason
// purely through the spec: the body is never expanded.
module 0x42::inline_opaque_hof_for_each_ref {
    use std::vector;

    /// `for_each_ref(v, f)` invokes `f` on each element in index order. The
    /// closure must be `copy + drop` since it is called once per slot.
    inline fun for_each_ref<T>(v: &vector<T>, f: |&T| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
            invariant forall j in 0..i: ensures_of<f>(v[j]);
        };
    }
    spec for_each_ref {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures forall i in 0..len(v): ensures_of<f>(v[i]);
    }

    /// Caller: assert every element is below a threshold (`assert` is encoded
    /// as `aborts_if` in the lambda; the caller's `requires` discharges it).
    fun check_all_below(v: &vector<u64>, hi: u64) {
        for_each_ref(v, |e| {
            assert!(*e < hi, 0)
        } spec {
            aborts_if e >= hi;
            ensures e < hi;
        });
    }
    spec check_all_below {
        requires forall i in 0..len(v): v[i] < hi;
        aborts_if false;
        ensures forall i in 0..len(v): v[i] < hi;
    }

    /// Caller: passing a lambda whose abort condition is discharged by a
    /// stronger `requires` on the caller. Demonstrates `aborts_of` flowing
    /// through the opaque HOF spec to the lambda spec.
    fun no_zero(v: &vector<u64>) {
        for_each_ref(v, |e| {
            assert!(*e != 0, 1)
        } spec {
            aborts_if e == 0;
            ensures e != 0;
        });
    }
    spec no_zero {
        requires forall i in 0..len(v): v[i] != 0;
        aborts_if false;
    }

    /// Caller: negative case. The required precondition is not provided by the
    /// caller, so the HOF's `requires` cannot be discharged at the call site.
    fun no_zero_unchecked(v: &vector<u64>) {
        for_each_ref(v, |e| { // error: precondition does not hold at this call
            assert!(*e != 0, 1)
        } spec {
            aborts_if e == 0;
            ensures e != 0;
        });
    }
    spec no_zero_unchecked {
        aborts_if false;
    }
}
