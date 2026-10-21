// A closure whose effects are not known cannot be checked against the frame a verified
// opaque callee declares (here, by omission, the empty one) for its parameter, and has to
// be rejected: otherwise the call havocs nothing and the false claims below verify. The
// Lean backend does not run this check and still accepts `deferring_target` (a known gap).
module 0x42::lambda_unknown_closure_declared_frame {
    struct R has key { v: u64 }

    fun op(h: |u64| u64 has copy + drop, y: u64): u64 {
        h(y)
    }
    spec op {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    // Writes R itself; the wildcard frame for f leaves its effects unknown.
    fun g(f: |u64| u64 has copy + drop, y: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = y;
        f(y)
    }
    spec g {
        modifies_of<f> *;
    }

    fun id(y: u64): u64 {
        y
    }

    fun wildcard_target(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let f0: |u64| u64 has copy + drop = |z| id(z);
        op(|y| g(f0, y), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec wildcard_target {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }

    // Leaves its parameter's frame to its call sites, so its own effects omit what f writes.
    fun apply(f: |u64| u64 has copy + drop, y: u64): u64 {
        f(y)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun writer(y: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = y;
        y
    }

    fun deferring_target(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let w: |u64| u64 has copy + drop = |z| writer(z);
        op(|y| apply(w, y), before + 1);
        borrow_global<R>(@0x42).v
    }
    spec deferring_target {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
