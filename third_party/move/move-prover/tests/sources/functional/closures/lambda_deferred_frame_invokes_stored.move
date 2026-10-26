// The same, with no function value captured: the closure's target invokes a callback it
// reads from global storage. That callback's writes are not in the target's summary, so
// the closure has no known footprint and handing it to a callee that leaves its frame to
// the call site must be rejected.
module 0x42::lambda_deferred_frame_invokes_stored {
    struct R has key { v: u64 }
    struct Cb has key { f: |u64| u64 has copy + drop + store }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    #[persistent]
    fun writer(y: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = y;
        y
    }
    spec writer {
        pragma opaque;
        modifies global<R>(@0x42);
        aborts_if !exists<R>(@0x42);
        ensures global<R>(@0x42).v == y;
        ensures result == y;
    }

    fun run(y: u64): u64 acquires Cb {
        let f = borrow_global<Cb>(@0x42).f;
        f(y)
    }

    fun caller(): u64 acquires R, Cb {
        let before = borrow_global<R>(@0x42).v;
        call_g(|y| run(y), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec caller {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
