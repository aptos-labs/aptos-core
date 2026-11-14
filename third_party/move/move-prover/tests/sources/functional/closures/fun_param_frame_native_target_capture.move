// A closure over a native or intrinsic whose function-typed parameter declares no frame has an
// unknown footprint, and what it captures for that parameter is checked against the empty
// frame: the native's contract says nothing about what it does through the parameter. Each
// client captures a writer of `S` into a closure over a native (with a non-opaque spec,
// without a spec, with an opaque spec), passes it to an opaque `T`-framed callee or to an
// opaque callee that leaves the frame to its call sites, and claims `S` unchanged, which is
// false at runtime; each capture must be rejected. See fun_param_frame_native_target_capture_ok
// for a native that declares the frame.
module 0x42::fun_param_frame_native_target_capture {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    fun bump_s(x: u64) acquires S { let s = &mut S[@0x42]; s.v = s.v + x; }
    spec bump_s {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + x > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + x;
    }

    native fun run_spec(f: |u64| has copy + drop, x: u64);
    spec run_spec {
        aborts_if false;
    }

    native fun run_bare(f: |u64| has copy + drop, x: u64);

    native fun run_opaque(f: |u64| has copy + drop, x: u64);
    spec run_opaque {
        pragma opaque;
        aborts_if false;
    }

    fun call_h(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_h {
        pragma opaque;
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun call_g(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun spec_declared() acquires S {
        let v0 = S[@0x42].v;
        let w: |u64| has copy + drop = |y| bump_s(y);
        call_h(|x| run_spec(w, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun spec_deferred() acquires S {
        let v0 = S[@0x42].v;
        let w: |u64| has copy + drop = |y| bump_s(y);
        call_g(|x| run_spec(w, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun bare_declared() acquires S {
        let v0 = S[@0x42].v;
        let w: |u64| has copy + drop = |y| bump_s(y);
        call_h(|x| run_bare(w, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun opaque_declared() acquires S {
        let v0 = S[@0x42].v;
        let w: |u64| has copy + drop = |y| bump_s(y);
        call_h(|x| run_opaque(w, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
