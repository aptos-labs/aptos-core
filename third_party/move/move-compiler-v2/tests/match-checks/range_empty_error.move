module 0xc0ffee::range_empty_error {
    // Empty range: exclusive range with equal bounds
    fun empty_exclusive(x: u8): u64 {
        match (x) {
            5..5 => 1,
            _ => 0,
        }
    }

    // Inverted exclusive range
    fun inverted_exclusive(x: u8): u64 {
        match (x) {
            10..5 => 1,
            _ => 0,
        }
    }

    // Inverted inclusive range
    fun inverted_inclusive(x: u8): u64 {
        match (x) {
            10..=5 => 1,
            _ => 0,
        }
    }

    // Empty signed range
    fun empty_signed(x: i8): u64 {
        match (x) {
            0i8..0i8 => 1,
            _ => 0,
        }
    }
}
