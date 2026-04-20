module 0xc0ffee::range_in_enum_non_exhaustive {
    enum Color has drop {
        RGB(u8, u8, u8),
        Named,
    }

    // Missing coverage: 128..=255 for first field
    fun missing_upper_range(c: Color): u64 {
        match (c) {
            Color::RGB(0..128, _, _) => 1,
            Color::Named => 2,
        }
    }

    enum E has drop {
        V1(u64),
        V2,
    }

    // Range in enum field without covering remaining values
    fun partial_range_in_enum(e: E): u64 {
        match (e) {
            E::V1(0..100) => 1,
            E::V2 => 2,
        }
    }
}
