module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    struct NoColor {
        red: u64
    }

    enum Fields {
        V1{f1: u64},
        V2{f1: u64, f2: u8},
        V3{f1: u8, f2: u8}
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

    fun select_variant_field_ok(self: Color): u64 {
        self.red
    }

    fun select_variant_field_err_multiple(self: Fields): u64 {
        self.f1 + (self.f2 as u64)
    }

}
