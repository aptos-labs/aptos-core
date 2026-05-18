// Tests decompilation of modules that define and consume public structs/enums.
// The compiler generates struct API wrapper functions (pack$S, unpack$S, etc.)
// in the binary for non-private structs; the decompiler must translate these
// back to native Move struct/enum operations.

module 0x42::defs {
    /// A public struct used from another module.
    public struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    /// A public enum used from another module.
    public enum Color has copy, drop {
        Red,
        Green { intensity: u64 },
        Blue,
    }
}

module 0x42::consumer {
    use 0x42::defs::{Point, Color};

    /// Pack a Point from another module (uses pack$Point API).
    fun make_point(x: u64, y: u64): Point {
        Point { x, y }
    }

    /// Unpack a Point from another module (uses unpack$Point API).
    fun sum_coords(p: Point): u64 {
        let Point { x, y } = p;
        x + y
    }

    /// Borrow a field of a Point (uses borrow$Point$x API).
    fun get_x(p: &Point): u64 {
        *&p.x
    }

    /// Pack an enum variant from another module (uses pack_variant$Color$Green API).
    fun make_green(intensity: u64): Color {
        Color::Green { intensity }
    }

    /// Test an enum variant from another module (uses test_variant$Color$Red API).
    fun is_red(c: &Color): bool {
        c is Color::Red
    }

    /// Unpack an enum variant from another module (uses unpack_variant$Color$Green API).
    fun get_green_intensity(c: Color): u64 {
        match (c) {
            Color::Green { intensity } => intensity,
            _ => 0,
        }
    }

    /// End-to-end test combining pack, borrow, and unpack.
    fun round_trip(): u64 {
        let p = make_point(3, 4);
        let x = get_x(&p);
        let total = sum_coords(p);
        x + total
    }
}
