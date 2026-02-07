module 0x815::types {
    public enum Color has copy, drop { Red { r: u8 }, Green { g: u16 }, Blue { b: u8 } }
    public fun make_color(r: u8): Color { Color::Blue { b: r } }
}
