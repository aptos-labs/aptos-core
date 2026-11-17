// A native higher-order function may invoke its function-typed parameters, so a call to it
// writes what their declared frames allow; a parameter without a frame makes the call's effects
// unknown. `t` invokes its pure captured `g`, then calls the native `run`, which declares
// `modifies_of<f> R2`, with a writer of R2: the closure `|y| t(w, y)` writes R2. Expected: each
// false claim that R2 is unchanged fails as a post-condition (the havoc covers R2). See
// fun_param_frame_reached_native_frames_ok.
module 0x42::fun_param_frame_reached_native_frames {
    struct R2 has key { v: u64 }

    native fun bump(y: u64);
    spec bump {
        pragma opaque;
        modifies global<R2>(@0x42);
        aborts_if false;
    }

    native fun run(f: |u64| has copy + drop, x: u64);
    spec run {
        pragma opaque;
        modifies_of<f>(_x: u64) R2[@0x42];
        aborts_if false;
    }

    fun pure_fn(z: u64) { let _ = z; }

    fun t(g: |u64| has copy + drop, x: u64) {
        g(x);
        run(|y| bump(y), x);
    }

    fun call_declared(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_declared {
        pragma opaque;
        pragma verify = false;
        modifies_of<f>(_x: u64) R2[@0x42];
        aborts_if false;
    }

    fun call_deferred(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_deferred {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun client_declared(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_declared(|y| t(w, y), x);
    }
    spec client_declared {
        pragma aborts_if_is_partial;
        ensures global<R2>(@0x42) == old(global<R2>(@0x42));
    }

    fun client_deferred(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_deferred(|y| t(w, y), x);
    }
    spec client_deferred {
        pragma aborts_if_is_partial;
        ensures global<R2>(@0x42) == old(global<R2>(@0x42));
    }
}
