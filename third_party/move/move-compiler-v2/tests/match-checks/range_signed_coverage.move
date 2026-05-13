module 0xc0ffee::range_signed_coverage {
    // Full i8 coverage with ranges crossing zero (should compile)
    fun full_i8_two_ranges(x: i8): u64 {
        match (x) {
            -128i8..0i8 => 1,
            0i8..=127i8 => 2,
        }
    }

    // Full i8 coverage with single inclusive range (should compile)
    fun full_i8_inclusive(x: i8): u64 {
        match (x) {
            -128i8..=127i8 => 1,
        }
    }

    // Range crossing zero with wildcard (should compile)
    fun crossing_zero_with_wildcard(x: i8): u64 {
        match (x) {
            -10i8..10i8 => 1,
            _ => 2,
        }
    }

    // Non-exhaustive: missing negative values
    fun missing_negatives(x: i8): u64 {
        match (x) {
            0i8..=127i8 => 1,
        }
    }

    // Non-exhaustive: gap crossing zero
    fun gap_crossing_zero(x: i8): u64 {
        match (x) {
            -128i8..-10i8 => 1,
            10i8..=127i8 => 2,
        }
    }
}
