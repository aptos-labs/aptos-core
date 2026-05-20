module 0xc0ffee::range_union_unreachable {
    // The first two ranges cover 5..15 as a union, so the third arm is dead.
    fun scalar_union_subsumes_range(x: u64): u64 {
        match (x) {
            0..10 => 1,
            10..20 => 2,
            5..15 => 3,
            _ => 4,
        }
    }

    // Same issue when the range sits in a tuple column.
    fun tuple_union_subsumes_range(x: u64, flag: bool): u64 {
        match ((x, flag)) {
            (0..10, false) => 1,
            (10..20, false) => 2,
            (5..15, false) => 3,
            (_, true) => 4,
            _ => 5,
        }
    }
}
