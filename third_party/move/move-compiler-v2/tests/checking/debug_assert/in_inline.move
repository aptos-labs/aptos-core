// Macro expansion survives inlining: each caller of `guarded` either inherits
// the assertion's abort path (test mode) or has no abort at all (non-test).
module 0x42::m {
    inline fun guarded(x: u64): u64 {
        debug_assert!(x > 0);
        x + 1
    }

    public fun caller_a(): u64 {
        guarded(5)
    }

    public fun caller_b(): u64 {
        guarded(10)
    }
}
