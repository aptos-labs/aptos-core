// Demonstrates that `pragma opaque` on an inline function causes the prover to
// substitute the SPEC at call sites instead of the inlined body.
//
// The body of `add_one_opaque` returns `x + 1`. The post-condition lies and
// says the result is `x + 2`. The caller's post-condition uses `2 + 41 = 43`.
//
// If the prover were inlining the body at the call site, `caller` would
// compute `41 + 1 = 42` and the post `result == 43` would fail. The fact that
// `caller` verifies proves the prover used the (lying) spec at the call site,
// not the body. Independently, the body of `add_one_opaque` is checked
// against its spec, and that check is expected to fail.
module 0x42::TestInlineFunSpecOpaque {

    spec module {
        pragma verify = true;
    }

    public inline fun add_one_opaque(x: u64): u64 {
        x + 1
    }
    spec add_one_opaque {
        pragma opaque = true;
        aborts_if x == 0xFFFFFFFFFFFFFFFF || x == 0xFFFFFFFFFFFFFFFE;
        ensures result == x + 2;
    }

    public fun caller(): u64 {
        add_one_opaque(41)
    }
    spec caller {
        ensures result == 43;
    }
}
