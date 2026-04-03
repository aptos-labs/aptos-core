module 0xc0ffee::range_overlap_reachability {
    // Partial overlap with different tuple columns: both reachable
    fun partial_overlap_both_reachable(x: u64, b: bool): u64 {
        match ((x, b)) {
            (0..10, true) => 1,
            (5..15, false) => 2,
            _ => 3,
        }
    }

    // Full overlap in multi-column: second arm unreachable
    fun full_overlap_unreachable(x: u64, b: bool): u64 {
        match ((x, b)) {
            (0..10, _) => 1,
            (5..8, true) => 2,
            _ => 3,
        }
    }

    // Three overlapping ranges where union covers a fourth
    fun union_covers_third(x: u64): u64 {
        match (x) {
            0..10 => 1,
            10..20 => 2,
            5..15 => 3,
            _ => 4,
        }
    }
}
