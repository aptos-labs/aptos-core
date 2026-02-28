module 0xc0ffee::m {
    enum Color {
        Red,
        Green,
        Blue,
        RGB(u8, u8, u8),
    }

    public fun test(c: &Color, p: bool, q: bool): u8 {
        match ((c, p)) {
            (Color::Red, true) => 0,
            (Color::Green, true) => 1,
            (Color::Blue, true) => 2,
            (Color::RGB(r, g, b), true) => 3,
            (Color::Red, false) if p => 4,
            (Color::RGB(r, g, b), false) if *r == 1 && q => 3,
            _ => 42
        }
    }
}
