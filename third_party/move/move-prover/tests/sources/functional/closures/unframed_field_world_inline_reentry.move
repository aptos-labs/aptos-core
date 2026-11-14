// Probe (d, VM premise): an inline function of L runs in its caller's frame, so L is NOT on the
// stack while the callback it invokes runs; the callback can call L::set_own without re-entry.
// run_t, run_o and witness_t are rejected (false at runtime); witness verifies only through the
// rejected summary of run_o.
module 0x42::L {
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    public fun make(f: |u64| u64 has copy + drop): Cb { Cb { f } }
    spec make { aborts_if false; }
    public fun get(c: &Cb): |u64| u64 has copy + drop { c.f }
    spec get { aborts_if false; ensures result == c.f; }
    public fun set_own(v: u64) acquires Own { if (exists<Own>(@0x42)) Own[@0x42].v = v; }
    spec set_own {
        pragma opaque;
        modifies global<Own>(@0x42);
        aborts_if false;
        ensures exists<Own>(@0x42) ==> global<Own>(@0x42).v == v;
        ensures exists<Own>(@0x42) == old(exists<Own>(@0x42));
    }

    // Non-inline control: true at runtime (L on the stack). Verifies expected.
    public fun run_n(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_n { ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }

    // Inline, transparent spec verified standalone. FALSE at runtime. Rejected expected.
    public inline fun run_t(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_t { ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }

    // Inline, opaque (retained, callers use the spec). FALSE at runtime. Rejected expected.
    public inline fun run_o(c: &Cb, x: u64): u64 { let g = get(c); g(x) }
    spec run_o { pragma opaque; ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }
}

module 0x42::X {
    use 0x42::L;
    // At runtime Own.v becomes x != old. Verifies only through a false summary.
    fun witness(x: u64): u64 {
        let c = L::make(|y| { L::set_own(y); y });
        L::run_o(&c, x)
    }
    spec witness {
        requires exists<L::Own>(@0x42) && global<L::Own>(@0x42).v != x;
        ensures global<L::Own>(@0x42) == old(global<L::Own>(@0x42));
    }
    // Same through the transparent inline.
    fun witness_t(x: u64): u64 {
        let c = L::make(|y| { L::set_own(y); y });
        L::run_t(&c, x)
    }
    spec witness_t {
        requires exists<L::Own>(@0x42) && global<L::Own>(@0x42).v != x;
        ensures global<L::Own>(@0x42) == old(global<L::Own>(@0x42));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
