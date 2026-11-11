// A native that declares a `T` frame for its function-typed parameter: a writer of `T` captured
// into a closure over it is accepted for a `T`-framed callee, and the claim that `S` is unchanged
// verifies. See fun_param_frame_native_target_capture.
module 0x42::fun_param_frame_native_target_capture_ok {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    fun bump_t(x: u64) acquires T { let t = &mut T[@0x42]; t.v = t.v + x; }
    spec bump_t {
        pragma opaque;
        modifies global<T>(@0x42);
        aborts_if !exists<T>(@0x42);
        aborts_if global<T>(@0x42).v + x > MAX_U64;
        ensures global<T>(@0x42).v == old(global<T>(@0x42).v) + x;
    }

    native fun run_t(f: |u64| has copy + drop, x: u64);
    spec run_t {
        modifies_of<f>(_x: u64) T[@0x42];
        aborts_if false;
    }

    fun call_h(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_h {
        pragma opaque;
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun client_declared() acquires S, T {
        let v0 = S[@0x42].v;
        let w: |u64| has copy + drop = |y| bump_t(y);
        call_h(|x| run_t(w, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
