// A closure handed to a callee that leaves its frame to the call site writes what its target's
// body writes, including through the functions encoded from their contracts that the body
// calls: their `modifies` frames are not in the body's usage summary. Each claim below that
// the resource is unchanged is false and must fail; the pure closure's claim holds.
module 0x42::fun_param_frame_deferred_reached {
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

    fun unverified_writer(_y: u64): u64 {
        abort 0
    }
    spec unverified_writer {
        pragma opaque;
        pragma verify = false;
        modifies global<R>(@0x42);
        aborts_if false;
        ensures global<R>(@0x42).v == _y;
        ensures result == _y;
    }

    fun native_in_body(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| native_writer(z) + 0, before + 1);
        borrow_global<R>(@0x42).v
    }
    spec native_in_body {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun unverified_in_body(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| unverified_writer(z) + 0, before + 1);
        borrow_global<R>(@0x42).v
    }
    spec unverified_in_body {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun pure_in_body(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| z + 1, before + 1);
        borrow_global<R>(@0x42).v
    }
    spec pure_in_body {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
