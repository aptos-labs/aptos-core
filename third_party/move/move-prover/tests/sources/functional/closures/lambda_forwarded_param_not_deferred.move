// Deferring the frame to the call site only works when the call site can answer the question.
// It can for a closure built there. A forwarded function parameter is not one: nothing at this
// call site knows what it writes, so nothing havocs it, and the caller could otherwise claim
// the memory survived. The argument must be rejected rather than deferred.
//
// Compare `regression/opaque_call_fun_arg_writes`, which is the same forwarding shape with the
// inner callee declaring a frame; there the frame propagates and the claim is refuted.
module 0x42::lambda_forwarded_param_not_deferred {
    struct R has key { value: u64 }

    fun apply(fv: |address| has drop, a: address) {
        fv(a)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
    }

    /// Must be rejected: `fv` is allowed to write `R[a]` by this function's own frame, but
    /// `apply` declares nothing and no closure is built here, so the write cannot be havoced.
    public fun forwarded(fv: |address| has drop, a: address) {
        apply(fv, a)
    }
    spec forwarded {
        modifies_of<fv>(x: address) R[x];
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }
}
