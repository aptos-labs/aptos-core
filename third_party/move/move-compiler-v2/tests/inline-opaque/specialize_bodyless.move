// A call to a spec function which cannot be specialized over a lambda
// argument (here: an uninterpreted spec function, which has no body to
// specialize) reports an error and must otherwise leave the call
// unchanged: substituting a boolean constant for it would be ill-typed
// whenever the function's result type is not bool.
module 0x42::specialize_bodyless {
    use std::vector;

    /// Uninterpreted: there is no body to specialize over the lambda.
    spec fun opaque_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, end: u64): u64;

    inline fun fold(v: &vector<u64>, init: u64, f: |u64, u64| u64): u64 {
        let acc = init;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            acc = f(acc, *vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant acc == opaque_fold(f, v, init, i); // error: cannot pass a lambda to native or uninterpreted spec function
        };
        acc
    }

    fun sum(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + e)
    }
    spec sum {
        pragma aborts_if_is_partial;
    }
}
