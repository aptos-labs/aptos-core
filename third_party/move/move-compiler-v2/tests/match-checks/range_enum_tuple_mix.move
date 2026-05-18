module 0xc0ffee::range_enum_tuple_mix {
    enum Color {
        Red,
        Green,
        Blue,
    }

    // Exhaustive: enum + range cover all cases
    fun exhaustive_enum_range(c: Color, x: u8): u64 {
        match ((c, x)) {
            (Color::Red, 0..128) => 1,
            (Color::Red, 128..=255) => 2,
            (Color::Green, _) => 3,
            (Color::Blue, _) => 4,
        }
    }

    // Non-exhaustive: missing Green with some range values
    fun non_exhaustive_enum_range(c: Color, x: u8): u64 {
        match ((c, x)) {
            (Color::Red, _) => 1,
            (Color::Green, 0..100) => 2,
            (Color::Blue, _) => 3,
        }
    }

    // Unreachable: range inside enum column is subsumed
    fun unreachable_enum_range(c: Color, x: u8): u64 {
        match ((c, x)) {
            (Color::Red, _) => 1,
            (Color::Red, 0..10) => 2,
            (_, _) => 3,
        }
    }
}
