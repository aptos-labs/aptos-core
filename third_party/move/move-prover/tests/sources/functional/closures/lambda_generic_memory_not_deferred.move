// A callee whose specification names memory only through a type parameter still constrains
// global memory, so it must not leave its function parameter's frame to the call site. Its
// `ensures` is assumed by every caller, and once instantiated it can re-establish exactly the
// claim the call-site havoc of the closure's writes was meant to discharge.
//
// Such memory is recorded in the generic specification-memory sets, not the ordinary ones, and
// the only expression that puts it there is `object::spec_exists_at<T>`, which the backend
// translates as `exists<T>`. This stub declares it the way the framework does, at the extlib
// address the harness binds to `extensions`.
module 0x2::object {
    spec module {
        fun spec_exists_at<T: key>(object: address): bool;
    }
}

module 0x42::lambda_generic_memory_not_deferred {
    use 0x2::object;

    struct R has key { n: u64 }

    /// Unverified, no frame for `f`, and its only mention of memory is through `T`.
    public fun apply<T: key>(_a: address, f: ||) {
        f()
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        ensures object::spec_exists_at<T>(_a);
    }

    /// Must be rejected: the closure removes `R`, and `apply` does not defer, so its empty frame
    /// stands. Were it deferred, `apply<R>`'s `ensures` would reassert `R`'s existence after the
    /// havoc and the false claim below would verify.
    public fun claims_still_exists(a: address) acquires R {
        apply<R>(a, || {
            let R { n: _ } = move_from<R>(a);
        })
    }
    spec claims_still_exists {
        pragma aborts_if_is_partial = true;
        requires exists<R>(a);
        ensures exists<R>(a);
    }
}
