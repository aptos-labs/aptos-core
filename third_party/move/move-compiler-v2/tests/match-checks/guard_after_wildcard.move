module 0xc0ffee::m {

    enum Color {
        Red,
        Green,
        Blue,
    }

    // A guarded arm after an unconditional wildcard is dead code.
    // Should be detected as unreachable (guards do not consume coverage).
    fun wildcard_then_guard(x: u64, cond: bool): u64 {
        match (x) {
            _ => 0,
            y if cond => 1,
        }
    }

    // Same with enum: all variants covered, then a guarded arm.
    fun exhaustive_then_guard(c: Color, cond: bool): u8 {
        match (c) {
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
            x if cond => 4,
        }
    }
}
