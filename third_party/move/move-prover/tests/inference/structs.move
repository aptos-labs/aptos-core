// Test spec inference for struct and enum operations
module 0x42::structs {

    // Simple struct
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    // Nested struct
    struct Rectangle has copy, drop {
        top_left: Point,
        bottom_right: Point,
    }

    // Enum type
    enum Color has copy, drop {
        Red,
        Green,
        Blue,
        RGB { r: u8, g: u8, b: u8 },
    }

    // Pack - should infer: ensures result == Point { x, y }
    fun make_point(x: u64, y: u64): Point {
        Point { x, y }
    }

    // Unpack - should infer: ensures result == p.x + p.y
    fun point_sum(p: Point): u64 {
        let Point { x, y } = p;
        x + y
    }

    // Nested pack - should infer nested pack expression
    fun make_rect(x1: u64, y1: u64, x2: u64, y2: u64): Rectangle {
        Rectangle {
            top_left: Point { x: x1, y: y1 },
            bottom_right: Point { x: x2, y: y2 },
        }
    }

    // Pack variant with data - should infer correct pack
    fun make_rgb(r: u8, g: u8, b: u8): Color {
        Color::RGB { r, g, b }
    }

    // Simple variant without data
    fun make_red(): Color {
        Color::Red
    }
}
