// A callee that leaves its closure's frame to the call site records nothing about the
// closure in its own summary. What the closure writes is written by the call, so it must
// reach every analysis that asks what a call or a function modifies: the summary of the
// function making the call (read by its callers, opaque or not, and by a closure built
// around it) and the havoc at a loop head that contains the call. Each claim below says
// the resource is unchanged, which is false, and must fail.
module 0x42::lambda_deferred_frame_propagates {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

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

    fun write_via_opaque(y: u64): u64 {
        call_g(|z| writer(z), y)
    }
    spec write_via_opaque {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    fun write_via_transparent(y: u64): u64 {
        call_g(|z| writer(z), y)
    }

    // Through an opaque function: its callers havoc what its summary says it modifies.
    fun opaque_helper(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        write_via_opaque(before + 1);
        borrow_global<R>(@0x42).v
    }
    spec opaque_helper {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    // Through two opaque layers.
    fun write_via_opaque2(y: u64): u64 {
        write_via_opaque(y)
    }
    spec write_via_opaque2 {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    fun nested_opaque_helper(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        write_via_opaque2(before + 1);
        borrow_global<R>(@0x42).v
    }
    spec nested_opaque_helper {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    // Through a closure whose target makes the deferred call: the closure's footprint is
    // its target's summary.
    fun closure_around_helper(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| write_via_transparent(z), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec closure_around_helper {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    // An opaque helper called in a loop: the loop head havocs what the callee modifies.
    fun opaque_helper_in_loop(n: u64): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let i = 0;
        while (i < n) {
            write_via_opaque(before + 1);
            i = i + 1;
        };
        borrow_global<R>(@0x42).v
    }
    spec opaque_helper_in_loop {
        pragma aborts_if_is_partial;
        requires n > 0;
        ensures result == old(global<R>(@0x42).v);
    }

    // The deferred call itself in a loop, with the closure built once before it.
    fun deferred_call_in_loop(n: u64): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let f: |u64| u64 has copy + drop = |z| writer(z);
        let i = 0;
        while (i < n) {
            call_g(f, before + 1);
            i = i + 1;
        };
        borrow_global<R>(@0x42).v
    }
    spec deferred_call_in_loop {
        pragma aborts_if_is_partial;
        requires n > 0;
        ensures result == old(global<R>(@0x42).v);
    }
}
