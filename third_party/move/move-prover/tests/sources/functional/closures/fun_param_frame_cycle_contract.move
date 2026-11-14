// exclude_for: lean
// The closure's target reaches a contract-encoded function of the enclosing function's own
// recursive cycle. The usage analysis has no summary for that function while it computes the
// cycle, so it records no footprint for the closure; the frame check must not accept one
// either, or the call would havoc writes the enclosing function's summary never gained.
// Expected: ERR at `enclosing`'s call to `deferred` (the false claim in `caller` is not
// verified).
module 0x42::frame_cycle_contract {
    struct R has key { v: u64 }

    fun enc(x: u64) acquires R {
        R[@0x42].v = x;
        if (x > 0) enclosing(x - 1)
    }
    spec enc { pragma opaque; pragma verify = false; modifies R[@0x42]; }

    fun t(x: u64) acquires R { enc(x) }

    // Leaves the frame of `f` to its call sites.
    fun deferred(f: |u64| has copy + drop, x: u64) { f(x) }
    spec deferred { pragma opaque; pragma verify = false; aborts_if false; }

    fun enclosing(x: u64) acquires R { deferred(|y| t(y), x) }
    spec enclosing { pragma opaque; pragma aborts_if_is_partial; }

    fun caller(x: u64) acquires R { enclosing(x) }
    spec caller {
        pragma aborts_if_is_partial;
        ensures R[@0x42] == old(R[@0x42]);
    }
}
