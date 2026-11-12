// Probe (d): inline (opaque-retained) function of L; callback state change through L itself.
// (1) control: claim about ANOTHER module's resource -> must be rejected on patched (World havoc happens).
// (2) ghost: L's spec var updated by L::bump which must access L::Own; kept by ghost_kept_in in L.
//     FALSE at runtime when run_g is inlined into X (L not on the stack). Rejected expected.
module 0x42::O { struct R has key { v: u64 } public fun set(v: u64) acquires R { if (exists<R>(@0x42)) R[@0x42].v = v; } }
module 0x42::L {
    use 0x42::O;
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    spec module { global counter: u64; }
    public fun make(f: |u64| u64 has copy + drop): Cb { Cb { f } }
    spec make { aborts_if false; }
    public fun get(c: &Cb): |u64| u64 has copy + drop { c.f }
    spec get { aborts_if false; ensures result == c.f; }
    public fun bump() acquires Own {
        Own[@0x42].v = Own[@0x42].v + 1;
        spec { update counter = counter + 1; };
    }
    spec bump { pragma opaque; modifies Own[@0x42]; aborts_if !exists<Own>(@0x42); aborts_if Own[@0x42].v + 1 > MAX_U64; ensures counter == old(counter) + 1; ensures Own[@0x42].v == old(Own[@0x42].v) + 1; }

    // (1) control. Rejected expected (another module's resource).
    public inline fun run_ctl(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_ctl { pragma opaque; ensures global<O::R>(@0x42) == old(global<O::R>(@0x42)); }

    // (2) ghost through an opaque inline. FALSE at runtime from X. Rejected expected.
    public inline fun run_g(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_g { pragma opaque; ensures counter == old(counter); }

    // (2') same, non-inline: true at runtime (L on the stack). Rejected, a precision loss: a direct
    // invocation keeps only the spec variables no unknown code can reach an update of.
    public fun run_gn(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_gn { pragma opaque; ensures counter == old(counter); }
}

module 0x42::X {
    use 0x42::L;
    // Runtime: L::bump runs with L off the stack, counter + 1. Verifies only through a false summary.
    fun witness_g(x: u64): u64 {
        let c = L::make(|y| { L::bump(); y });
        L::run_g(&c, x)
    }
    spec witness_g { requires exists<L::Own>(@0x42) && global<L::Own>(@0x42).v < 100; ensures L::counter == old(L::counter); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
