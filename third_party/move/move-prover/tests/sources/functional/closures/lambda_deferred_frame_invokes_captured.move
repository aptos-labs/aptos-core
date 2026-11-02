// A closure whose target invokes a function value it receives has no known footprint: the
// target's summary records what that value writes only as far as a `modifies_of`
// declaration bounds it, and here none does. Handing the closure to a callee that leaves
// its frame to the call site must be rejected; deferring it would havoc only what the
// target writes itself, and the claim that R is unchanged would verify.
module 0x42::lambda_deferred_frame_invokes_captured {
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

    fun both(h: |u64| u64 has copy + drop, y: u64): u64 acquires S {
        borrow_global_mut<S>(@0x42).v = y;
        h(y)
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

    fun caller(): u64 acquires R, S {
        let before = borrow_global<R>(@0x42).v;
        let h: |u64| u64 has copy + drop = |z| writer(z);
        call_g(|y| both(h, y), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec caller {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
