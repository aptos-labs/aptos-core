// A bodyful spec function that forwards a lambda-bound function parameter
// to an uninterpreted spec function: the outer specialization succeeds
// structurally, but the forwarded call cannot be specialized (there is no
// body). The failed inner specialization must invalidate the outer one as
// well, so the call site takes the same leave-intact path as a direct call
// to the uninterpreted function; otherwise the registered outer
// specialization would retain a dangling reference to its eliminated
// function parameter.
module 0x42::specialize_forward_bodyless {
    use std::vector;

    /// Uninterpreted: there is no body to specialize over the lambda.
    spec fun opaque_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, end: u64): u64;

    /// Bodyful: forwards `f` to the uninterpreted function.
    spec fun spec_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, end: u64): u64 {
        opaque_fold(f, v, init, end) // error: cannot pass a lambda to native or uninterpreted spec function
    }

    inline fun fold(v: &vector<u64>, init: u64, f: |u64, u64| u64): u64 {
        let acc = init;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            acc = f(acc, *vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant acc == spec_fold(f, v, init, i);
        };
        acc
    }

    fun sum(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + e)
    }
    spec sum {
        pragma aborts_if_is_partial;
    }

    /// A second expansion with a spec-equivalent lambda: the failed
    /// specialization is unified, so no second attempt is made and the
    /// error is reported only once.
    fun sum_again(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + e)
    }
    spec sum_again {
        pragma aborts_if_is_partial;
    }
}
