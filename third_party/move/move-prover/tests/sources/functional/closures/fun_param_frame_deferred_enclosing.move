// A closure handed to a callee that leaves its frame to the call site, whose target writes R and
// calls back into the function building the closure. The usage analysis summarizes that function
// while its body is absent, so it cannot account for the closure; the frame check must agree and
// reject the call, or the opaque function's summary omits R and the false claim in `h` verifies.
module 0x42::fun_param_frame_deferred_enclosing {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun back(x: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = 9;
        f(x)
    }

    fun f(n: u64): u64 {
        if (n == 0) { 0 } else { call_g(back, n - 1) }
    }
    spec f {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    public fun h(): u64 acquires R {
        f(1);
        borrow_global<R>(@0x42).v
    }
    spec h {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
