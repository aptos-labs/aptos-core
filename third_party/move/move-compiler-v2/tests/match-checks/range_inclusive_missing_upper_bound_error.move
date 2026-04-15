module 0xc0ffee::range_inclusive_missing_upper_bound_error {
    // `..=` should not be accepted without an upper bound.
    fun open_start_inclusive_without_hi(x: u64): u64 {
        match (x) {
            ..= => 1,
            _ => 0,
        }
    }

    // `lo..=` should not be accepted without an upper bound either.
    fun open_end_inclusive_without_hi(x: u64): u64 {
        match (x) {
            0..= => 1,
            _ => 0,
        }
    }
}
