// flag: --verify-only=opaque_conditionless_callee::caller
// An opaque callee whose contract states no condition, like `event::emit`:
// WP names it through `ensures_of`, and the prover must accept that, since
// callers are verified against the contract however little it says. The
// contract leaves its aborts open, so the caller's stay partial.
module 0x42::opaque_conditionless_callee {
    fun record(x: u64) {
        let _ = x;
    }
    spec record {
        pragma opaque;
    }

    fun caller(x: u64): u64 {
        record(x);
        x
    }
}
