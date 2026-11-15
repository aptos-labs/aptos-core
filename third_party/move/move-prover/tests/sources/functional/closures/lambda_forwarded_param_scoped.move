// flag: --verify-only=forwarded
// The other half of `lambda_frame_deferral_scoped`: scoping a run must not make a rejected
// call acceptable either. This is `lambda_forwarded_param_not_deferred` under
// `--verify-only`, and it has to stay rejected. Deciding deferral from the verification
// scope would silently accept it here, since nothing is explicitly unverified once the scope
// names a single function.
module 0x42::lambda_forwarded_param_scoped {
    struct R has key { value: u64 }

    fun apply(fv: |address| has drop, a: address) {
        fv(a)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
    }

    /// Must be rejected: `fv` may write `R[a]` and nothing here can havoc that write.
    public fun forwarded(fv: |address| has drop, a: address) {
        apply(fv, a)
    }
    spec forwarded {
        modifies_of<fv>(x: address) R[x];
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }
}
