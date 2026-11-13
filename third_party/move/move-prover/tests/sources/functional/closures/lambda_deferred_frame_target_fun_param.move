// A closure whose target has a function-typed parameter has no known footprint. The target's
// summary bounds what invoking that parameter writes by the check a direct call makes on its
// argument (here, against the empty frame of an opaque function), but a closure's target is
// not called directly: the value comes from the closure's captures (`caller`) or from whoever
// invokes the closure (`uncaptured`, `uncaptured_eta`, where the deferring callee's unverified
// body supplies a writer), and neither is checked. Handing such a closure to a callee that
// leaves its frame to the call site must be rejected; deferring it would havoc only what the
// target writes itself, and the claims that R or C is unchanged would verify.
module 0x42::lambda_deferred_frame_target_fun_param {
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
    spec both {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies global<S>(@0x42);
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

    struct C has key { n: u64 }

    fun bump(a: address) acquires C {
        C[a].n = C[a].n + 1;
    }

    fun t(f: |address| has drop, a: address) acquires S {
        borrow_global_mut<S>(a).v = 0;
        f(a)
    }
    spec t {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies global<S>(a);
    }

    fun d(g: |(|address| has drop), address| has drop, a: address) {
        g(|y| bump(y), a)
    }
    spec d {
        pragma opaque;
        pragma verify = false;
    }

    fun uncaptured(a: address) {
        d(|f, x| t(f, x), a)
    }
    spec uncaptured {
        pragma aborts_if_is_partial;
        ensures global<C>(a).n == old(global<C>(a).n);
    }

    fun uncaptured_eta(a: address) {
        d(t, a)
    }
    spec uncaptured_eta {
        pragma aborts_if_is_partial;
        ensures global<C>(a).n == old(global<C>(a).n);
    }
}
