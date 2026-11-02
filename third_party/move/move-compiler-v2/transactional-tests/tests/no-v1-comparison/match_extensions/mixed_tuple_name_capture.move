//# publish
module 0xc0ffee::m {
    // Mixed-tuple lowering generates tests and bindings inside arm scopes.
    // Discriminator references in those expressions must retain their meaning
    // when pattern variables shadow the discriminator's original names.
    enum W has copy, drop { W1(u64), W2(u64) }
    enum E has copy, drop { V { x: u64 }, N }

    public fun capture_literal(v: u64, kk: u64): u64 {
        let w = W::W1(v);
        let k = kk;
        match ((w, k)) {
            (W::W1(k), 5) => k,
            _ => 0,
        }
    }

    public fun capture_binding(v: u64, kk: u64): u64 {
        let w = W::W1(v);
        let k = kk;
        match ((w, k)) {
            (W::W1(k), y) if (y == 5) => k,
            (W::W1(k), y) => y * 100 + k,
            _ => 0,
        }
    }

    // Swapped pattern names must still read the original discriminator values.
    public fun capture_swap(a0: u64, b0: u64): u64 {
        let w = W::W1(0);
        let a = a0;
        let b = b0;
        match ((w, a, b)) {
            (W::W1(_), b, a) => a * 100 + b,
            _ => 0,
        }
    }

    // Parameters become named locals when inlined.
    inline fun pick(e: E, x: u64): u64 {
        match ((e, x)) {
            (E::V { x }, 5) => x,
            _ => 0,
        }
    }
    public fun capture_after_inlining(): u64 { pick(E::V { x: 5 }, 3) }
}

//# run 0xc0ffee::m::capture_literal --args 5u64 7u64

//# run 0xc0ffee::m::capture_literal --args 7u64 5u64

//# run 0xc0ffee::m::capture_binding --args 7u64 5u64

//# run 0xc0ffee::m::capture_binding --args 7u64 3u64

//# run 0xc0ffee::m::capture_swap --args 1u64 2u64

//# run 0xc0ffee::m::capture_after_inlining
