module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    fun test_rgb_reduces_to_is_cascade_with_opt(self: &Color): bool {
        match (self) {
            Red => false,
            Blue => false,
            RGB{..} => true
        }
    }

    fun test_red(self: &Color): bool {
        match (self) {
            Red => true,
            _ => false
        }
    }

}
