// A direct call to an opaque function under `verify = false` havocs what its body writes
// outside its `modifies` frame, as well as the frame itself. A closure over such a function,
// handed to a callee that leaves its frame to the call site, writes the same. Each claim
// below says the body-written resource is unchanged after the closure runs, which is false,
// and must fail. The Lean backend does not see the body writes and still accepts both (a
// known gap).
module 0x42::lambda_deferred_frame_unframed_body {
    struct R has key { v: u64 }
    struct S has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // No frame; the body writes R.
    fun unframed_writer(y: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = y;
        y
    }
    spec unframed_writer {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // The frame names R; the body also writes S.
    fun partly_framed_writer(y: u64): u64 acquires R, S {
        borrow_global_mut<R>(@0x42).v = y;
        borrow_global_mut<S>(@0x42).v = y;
        y
    }
    spec partly_framed_writer {
        pragma opaque;
        pragma verify = false;
        modifies global<R>(@0x42);
        aborts_if false;
    }

    fun unframed(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        call_g(|z| unframed_writer(z), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec unframed {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    fun partly_framed(): u64 acquires R, S {
        let before = borrow_global<S>(@0x42).v;
        call_g(|z| partly_framed_writer(z), before + 1);
        borrow_global<S>(@0x42).v
    }
    spec partly_framed {
        pragma aborts_if_is_partial;
        ensures result == old(global<S>(@0x42).v);
    }
}
