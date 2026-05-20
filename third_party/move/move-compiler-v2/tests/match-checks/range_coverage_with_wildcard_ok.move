module 0xc0ffee::range_coverage_with_wildcard_ok {
    // Range + wildcard is exhaustive
    fun range_with_wildcard(x: u64): u64 {
        match (x) {
            0..100 => 1,
            _ => 2,
        }
    }

    // Multiple ranges + wildcard
    fun multi_range_with_wildcard(x: u8): u64 {
        match (x) {
            0..10 => 1,
            10..20 => 2,
            _ => 3,
        }
    }

    // Open-ended ranges covering all values
    fun open_ranges_full_coverage(x: u64): u64 {
        match (x) {
            ..50 => 1,
            50.. => 2,
        }
    }

    // Inclusive range covering full u8 range
    fun full_u8_range(x: u8): u64 {
        match (x) {
            0..=255 => 1,
        }
    }

    enum E has drop {
        V1(u64),
        V2,
    }

    // Range in enum + wildcard for remaining
    fun enum_range_with_wildcard(e: E): u64 {
        match (e) {
            E::V1(0..100) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }
}
