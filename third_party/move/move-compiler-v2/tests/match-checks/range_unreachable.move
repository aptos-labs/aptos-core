module 0xc0ffee::range_unreachable {
    // Subsumed range: 3..7 is within 1..10
    fun subsumed_range(x: u64): u64 {
        match (x) {
            1..10 => 1,
            3..7 => 2,
            _ => 0,
        }
    }

    // Wildcard followed by range
    fun wildcard_then_range(x: u64): u64 {
        match (x) {
            _ => 1,
            1..10 => 2,
        }
    }

    // Full coverage followed by anything
    fun full_range_then_literal(x: u64): u64 {
        match (x) {
            _ => 1,
            5 => 2,
        }
    }

    // Inclusive range subsumes literal
    fun range_subsumes_literal(x: u64): u64 {
        match (x) {
            1..=10 => 1,
            5 => 2,
            _ => 0,
        }
    }

    // Duplicate identical ranges
    fun duplicate_ranges(x: u64): u64 {
        match (x) {
            1..10 => 1,
            1..10 => 2,
            _ => 0,
        }
    }
}
