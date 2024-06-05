module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    struct NoColor {
        red: u64
    }

    fun unqualified_variant(self: Color): bool {
        match (self) {
            // We may want to fix this later and allow it if it can be
            // derived from the context type.
            RGB{red, green, blue} => false,
            Color::Red => true,
            Color::Blue => false,
        }
    }

    fun misspelled_variant(self: Color): bool {
        match (self) {
            Color::Rgb{red, green, blue} => false,
            Color::Red => true,
            Color::Blue => false,
        }
    }

    fun missplaced_variant(self: Color::Red): bool {
        0x815::m::missplaced_variant::Red();
        false
    }

    fun missing_field(self: Color::Red): bool {
        match (self) {
            Color::RGB{red, green} => false,
        }
    }

    fun extra_field(self: Color): bool {
        match (self) {
            Color::RGB{red, green, blue, black} => false,
        }
    }

    fun select_variant_field(self: Color): u64 {
        self.red
    }
}
