module 0x42::basic {
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    fun add(a: u64, b: u64): u64 {
        a + b
    }

    fun make_point(x: u64, y: u64): Point {
        Point { x, y }
    }

    fun get_x(p: &Point): u64 {
        p.x
    }

    fun max(a: u64, b: u64): u64 {
        if (a > b) { a } else { b }
    }
}
