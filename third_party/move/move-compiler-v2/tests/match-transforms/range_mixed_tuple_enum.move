module 0xc0ffee::m {
    enum Color has drop {
        RGB(u8, u8, u8),
        Grayscale(u8),
        Named,
    }

    // Range inside enum fields + range in tuple position
    fun mixed_range_tuple(c: Color, x: u64): u64 {
        match ((c, x)) {
            (Color::RGB(0..25, 28, 55..66), 88..99) => 1,
            (Color::RGB(_, _, _), _) => 2,
            (Color::Grayscale(128..=255), 0..50) => 3,
            (Color::Grayscale(_), _) => 4,
            (Color::Named, _) => 5,
        }
    }
}
