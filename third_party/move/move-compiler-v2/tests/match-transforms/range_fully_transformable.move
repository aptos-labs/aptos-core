module 0xc0ffee::m {
    // Exclusive ranges: 0..5 lowers to (0 <= x && x < 5)
    public fun exclusive_ranges(p: u8): u8 {
        match (p) {
            0..5 => 1,
            5..10 => 2,
            _ => 3,
        }
    }

    // Inclusive range: 1..=5 lowers to (1 <= x && x <= 5)
    public fun inclusive_range(p: u64): u64 {
        match (p) {
            1..=5 => 1,
            6..=10 => 2,
            _ => 3,
        }
    }

    // Open-ended range: 5.. lowers to (5 <= x)
    public fun open_end_range(p: u64): u64 {
        match (p) {
            ..5 => 1,
            5.. => 2,
        }
    }

    // Open-start inclusive: ..=5 lowers to (x <= 5)
    public fun open_start_inclusive(p: u64): u64 {
        match (p) {
            ..=5 => 1,
            6.. => 2,
        }
    }

    // Mixed ranges and literals
    public fun mixed_range_literal(p: u8): u8 {
        match (p) {
            0 => 1,
            1..10 => 2,
            10 => 3,
            _ => 4,
        }
    }

    // Range on signed integer
    public fun signed_range(p: i64): u64 {
        match (p) {
            -100..-1 => 1,
            0..=0 => 2,
            1..100 => 3,
            _ => 4,
        }
    }
}
