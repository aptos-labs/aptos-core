// exclude_for: lean
// The deferral shape of fun_param_frame_captured_declared. Expected: the false claim fails as a
// post-condition. Verex does not encode a frame left to the call site.

// `call_g` leaves the frame to the call site, which havocs the closure's footprint.
module 0x42::fun_param_frame_captured_deferred {
    struct R2 has key { v: u64 }

    native fun bump(y: u64);
    spec bump {
        pragma opaque;
        modifies global<R2>(@0x42);
        aborts_if false;
    }

    fun pure_fn(z: u64) { let _ = z; }

    fun t(g: |u64| has copy + drop, x: u64) {
        g(x);
        bump(x);
    }

    fun call_g(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // False: the closure writes R2.
    fun client(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_g(|y| t(w, y), x);
    }
    spec client {
        pragma aborts_if_is_partial;
        ensures global<R2>(@0x42) == old(global<R2>(@0x42));
    }
}
