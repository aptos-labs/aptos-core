module 0x815::m {
    use 0x815::m;

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    fun match_no_comma(self: Color): bool {
        match (self) {
            Color::Red => true
            Color::Blue => false,
        }
    }

}
