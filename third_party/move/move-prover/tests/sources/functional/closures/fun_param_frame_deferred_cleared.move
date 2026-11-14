// A closure handed to a callee that leaves its frame to the call site, over a private opaque
// wrapper of a native writer: the wrapper's usage summary omits the native's `modifies` frame,
// which the closure's footprint must still include. The claim that the resource is unchanged is
// false and must fail.
module 0x42::fun_param_frame_deferred_cleared {
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

    fun wrapper(y: u64): u64 {
        native_writer(y)
    }
    spec wrapper {
        pragma opaque;
        aborts_if false;
    }

    public fun caller(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(wrapper, before + 1);
        borrow_global<R>(@0x42).v
    }
    spec caller {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
