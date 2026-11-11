// Dependency of fun_param_frame_dependency and fun_param_frame_dependency_capture. Compiled as
// a dependency, no body here is frame-checked, so `apply` and `lam` keep an unbounded invoke.
// As a target on its own: ERR at `wrapper`'s call to `apply` (empty frame of `f`), and at the
// capture of `w` (a native writer) for `lam`'s `g` in `wrapper_capture`.
module 0x42::frame_dep {
    struct R has key { v: u64 }

    public fun get(): u64 acquires R { R[@0x42].v }

    native fun native_writer(x: u64);
    spec native_writer {
        pragma opaque;
        modifies global<R>(@0x42);
    }

    // Transparent; no frame declared for `f`.
    public fun apply(f: |u64| has drop) { f(0) }

    // Transparent; takes a function value, which a closure can capture.
    public fun lam(g: |u64| has drop, x: u64) { g(x) }

    public fun wrapper(x: u64): u64 { apply(native_writer); x }

    public fun wrapper_capture(x: u64): u64 {
        let w = native_writer;
        apply(|y| lam(w, y));
        x
    }
}
