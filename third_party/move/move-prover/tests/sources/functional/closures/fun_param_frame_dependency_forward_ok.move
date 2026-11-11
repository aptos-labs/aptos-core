// A forwarder whose transitive users are all in target modules keeps a bounded invoke:
// `apply`'s invoke is bounded, so the closure over `pure_wrapper` has a known, empty footprint
// and the true claim verifies (rejected when `apply`'s undeclared parameter is unbounded).
module 0x42::frame_fwd_ok {
    struct R has key { v: u64 }

    // Transparent; no frame declared for `f`.
    fun apply(f: |u64| has drop) { f(0) }

    fun fwd(g: |u64| has drop) { apply(g) }

    fun pure_wrapper(x: u64): u64 { fwd(|_y| ()); x }

    // Leaves the frame of `g` to its call sites.
    fun call_g(g: |u64| u64 has copy + drop, x: u64): u64 { g(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    public fun h(): u64 { call_g(pure_wrapper, 1) }
    spec h {
        pragma aborts_if_is_partial;
        ensures global<R>(@0x42).v == old(global<R>(@0x42).v);
    }
}
