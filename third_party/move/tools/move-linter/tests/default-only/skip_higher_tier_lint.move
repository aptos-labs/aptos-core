module 0xc0ffee::m {
    // Skip an experimental-only lint while running at default tier.
    // This should NOT produce an "unknown lint check" error.
    #[lint::skip(cyclomatic_complexity)]
    public fun test(): u64 { 1 }
}
