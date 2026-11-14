// A spec `let` evaluated at entry reads the memory saved for the entry state, which must be saved
// before the let is bound. `r0` is what the first invocation returns, in the entry state.
module 0x42::el {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop, g: |u64| u64 has copy + drop }
    struct Cf has drop, copy { f: |u64| u64 has copy + drop, g: |u64| u64 has copy + drop }
    spec Cf {
        reads_of<f> R;
        modifies_of<g>(y: u64) R[@0x42];
    }

    // The guard inline. Verifies.
    public fun inline_guard(c: &Cb, x: u64): u64 {
        let r = (c.f)(x);
        let _s = (c.g)(x);
        if (r == 0) abort 7;
        r
    }
    spec inline_guard {
        pragma aborts_if_is_partial;
        aborts_if aborts_of<c.f>(x);
        aborts_if !aborts_of<c.f>(x) && result_of<c.f>(x) == 0;
    }

    // The guard through a spec `let`. Verifies.
    public fun let_guard(c: &Cb, x: u64): u64 {
        let r = (c.f)(x);
        let _s = (c.g)(x);
        if (r == 0) abort 7;
        r
    }
    spec let_guard {
        pragma aborts_if_is_partial;
        let r0 = result_of<c.f>(x);
        aborts_if aborts_of<c.f>(x);
        aborts_if !aborts_of<c.f>(x) && r0 == 0;
    }

    // The same over framed fields: `f` reads `R`, `g` writes it. Verifies.
    public fun let_guard_framed(c: &Cf, x: u64): u64 {
        let r = (c.f)(x);
        let _s = (c.g)(x);
        if (r == 0) abort 7;
        r
    }
    spec let_guard_framed {
        pragma aborts_if_is_partial;
        let r0 = result_of<c.f>(x);
        aborts_if aborts_of<c.f>(x);
        aborts_if !aborts_of<c.f>(x) && r0 == 0;
    }

    // A `let` over a resource the invocation may change. Verifies.
    public fun let_global(c: &Cb, x: u64): u64 acquires R {
        let r = R[@0x42].v;
        let _s = (c.f)(x);
        if (r == 0) abort 7;
        r
    }
    spec let_global {
        pragma aborts_if_is_partial;
        requires exists<R>(@0x42);
        let v0 = global<R>(@0x42).v;
        aborts_if v0 == 0;
    }

    // False: the guard claims the opposite value. Rejected.
    public fun let_guard_false(c: &Cb, x: u64): u64 {
        let r = (c.f)(x);
        let _s = (c.g)(x);
        if (r == 0) abort 7;
        r
    }
    spec let_guard_false {
        pragma aborts_if_is_partial;
        let r0 = result_of<c.f>(x);
        aborts_if aborts_of<c.f>(x);
        aborts_if !aborts_of<c.f>(x) && r0 == 1;
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
