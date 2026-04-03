module 0xc0ffee::range_non_exhaustive {
    // Missing values outside range
    fun missing_outside_range(x: u64): u64 {
        match (x) {
            1..10 => 1,
        }
    }

    // Multiple non-overlapping ranges without full coverage
    fun gaps_between_ranges(x: u8): u64 {
        match (x) {
            0..10 => 1,
            20..30 => 2,
        }
    }

    // Open-ended range missing lower values
    fun missing_lower(x: u64): u64 {
        match (x) {
            5.. => 1,
        }
    }

    // Open-start range missing upper values
    fun missing_upper(x: u64): u64 {
        match (x) {
            ..100 => 1,
        }
    }
}
