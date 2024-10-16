module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    fun test_red_or_rgb(c: Color): bool {
        c is Red
    }
}
