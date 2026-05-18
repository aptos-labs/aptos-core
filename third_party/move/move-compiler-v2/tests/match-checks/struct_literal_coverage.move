module 0xc0ffee::struct_literal_coverage {
    struct S has drop { x: u64, y: u64 }

    // Non-exhaustive: missing non-42 values of x
    public fun non_exhaustive(s: S): u64 {
        match (s) {
            S { x: 42, y } => y,
        }
    }

    // Unreachable: S { x: 42, .. } is subsumed by S { x: _, .. }
    public fun unreachable(s: S): u64 {
        match (s) {
            S { x: _, y } => y,
            S { x: 42, y } => y + 1,
        }
    }

    // Exhaustive with literal (should pass with no errors)
    public fun exhaustive(s: S): u64 {
        match (s) {
            S { x: 42, y } => y + 100,
            S { x: _, y } => y,
        }
    }
}
