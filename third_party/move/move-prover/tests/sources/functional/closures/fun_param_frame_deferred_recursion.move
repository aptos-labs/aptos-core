// A closure handed to a callee that leaves its frame to the call site, whose target calls back
// into the function being verified: that function's body is not available while it is
// processed, so what the closure reaches through it (here a native writer) is unknown, and the
// closure has no footprint. The claim that `f` leaves R unchanged when `n > 0` is false and must
// not verify.
module 0x42::fun_param_frame_deferred_recursion {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    native fun native_writer(y: u64): u64;
    spec native_writer {
        pragma opaque;
        modifies global<R>(@0x42);
        aborts_if false;
        ensures global<R>(@0x42).v == y;
        ensures result == y;
    }

    fun back(x: u64): u64 {
        f(x)
    }

    fun f(n: u64): u64 {
        if (n == 0) {
            native_writer(7)
        } else {
            call_g(back, n - 1)
        }
    }
    spec f {
        pragma aborts_if_is_partial;
        ensures n > 0 ==> global<R>(@0x42) == old(global<R>(@0x42));
    }
}
