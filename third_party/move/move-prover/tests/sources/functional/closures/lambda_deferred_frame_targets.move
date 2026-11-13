// A closure handed to a callee that leaves its frame to the call site writes whatever the
// function it calls writes, as the prover sees a call to that function. For a function
// whose body the prover checks, that is the body's writes; for a native or an opaque
// function under `verify = false`, it is the contract's `modifies` frame, whatever the body
// does. Each claim below says the resource is unchanged after the closure runs, which is
// false, and must fail -- whether the call is made directly, inside an opaque helper, or in
// a loop.
module 0x42::lambda_deferred_frame_targets {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // Writes R through a native whose only account of the write is its contract.
    native fun native_writer(y: u64): u64;
    spec native_writer {
        pragma opaque;
        modifies global<R>(@0x42);
        aborts_if false;
        ensures global<R>(@0x42).v == y;
        ensures result == y;
    }

    // Writes R through an unverified opaque function whose body does not write at all.
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

    // Writes R through a mutually recursive pair.
    fun ping(n: u64, y: u64): u64 acquires R {
        if (n == 0) {
            borrow_global_mut<R>(@0x42).v = y;
            y
        } else {
            pong(n - 1, y)
        }
    }
    fun pong(n: u64, y: u64): u64 acquires R {
        ping(n, y)
    }

    fun native_direct(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| native_writer(z), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec native_direct {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun unverified_direct(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| unverified_writer(z), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec unverified_direct {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun native_in_helper(y: u64): u64 {
        call_g(|z| native_writer(z), y)
    }
    spec native_in_helper {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    fun native_through_helper(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        native_in_helper(before + 1);
        borrow_global<R>(@0x42).v
    }
    spec native_through_helper {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun unverified_in_loop(n: u64): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let f: |u64| u64 has copy + drop = |z| unverified_writer(z);
        let i = 0;
        while (i < n) {
            call_g(f, before + 1);
            i = i + 1;
        };
        borrow_global<R>(@0x42).v
    }
    spec unverified_in_loop {
        pragma aborts_if_is_partial;
        requires n > 0;
        ensures result == old(global<R>(@0x42).v);
    }

    fun recursive_direct(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| ping(1, z), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec recursive_direct {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
