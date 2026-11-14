// Dependency of fun_param_frame_captured_declared_dep: the closure `|y| t(w, y)` is built in a
// dependency body, where the capture mask does not count, so its footprint is not known.
// As a target on its own it verifies (no claim).
module 0x42::captured_declared_dep {
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
        modifies_of<f>(_x: u64) R2[@0x42];
        aborts_if false;
    }

    public fun wrapper(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_g(|y| t(w, y), x);
    }
}
