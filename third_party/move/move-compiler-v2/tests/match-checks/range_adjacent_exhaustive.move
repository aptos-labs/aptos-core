module 0xc0ffee::range_adjacent_exhaustive {
    // Two adjacent ranges covering full u8 (should compile)
    fun two_adjacent_u8(x: u8): u64 {
        match (x) {
            0..128 => 1,
            128..=255 => 2,
        }
    }

    // Three adjacent ranges covering full u8 (should compile)
    fun three_adjacent_u8(x: u8): u64 {
        match (x) {
            0..100 => 1,
            100..200 => 2,
            200..=255 => 3,
        }
    }

    // Literal + range coverage on u8 (should compile)
    fun literal_plus_range_u8(x: u8): u64 {
        match (x) {
            0 => 1,
            1..=255 => 2,
        }
    }
}
