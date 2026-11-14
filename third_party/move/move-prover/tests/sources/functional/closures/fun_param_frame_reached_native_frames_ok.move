// `run` declares R2 for its parameter, the outer frame covers R2, and the claim is about R3,
// which nothing writes: it verifies. See fun_param_frame_reached_native_frames.
module 0x42::fun_param_frame_reached_native_frames_ok {
    struct R2 has key { v: u64 }
    struct R3 has key { v: u64 }

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

    fun client_declared(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_declared(|y| t(w, y), x);
    }
    spec client_declared {
        pragma aborts_if_is_partial;
        ensures global<R3>(@0x42) == old(global<R3>(@0x42));
    }
}
