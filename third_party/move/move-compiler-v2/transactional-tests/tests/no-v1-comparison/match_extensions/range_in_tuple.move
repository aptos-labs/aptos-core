//# publish
module 0xc0ffee::m {
    // Ranges in both tuple positions
    public fun test_tuple_both_ranges(x: u64, y: u64): u64 {
        match ((x, y)) {
            (1..5, 10..20) => 1,
            _ => 0,
        }
    }

    // Mix of range and literal in tuple
    public fun test_tuple_range_and_literal(x: u64, y: u64): u64 {
        match ((x, y)) {
            (1..10, 42) => 1,
            (1..10, _) => 2,
            _ => 0,
        }
    }

    // Mix of range and wildcard in tuple
    public fun test_tuple_range_and_wildcard(x: u64, y: u64): u64 {
        match ((x, y)) {
            (0..100, _) => 1,
            _ => 2,
        }
    }
}

//# run 0xc0ffee::m::test_tuple_both_ranges --args 3u64 15u64

//# run 0xc0ffee::m::test_tuple_both_ranges --args 3u64 25u64

//# run 0xc0ffee::m::test_tuple_both_ranges --args 0u64 15u64

//# run 0xc0ffee::m::test_tuple_range_and_literal --args 5u64 42u64

//# run 0xc0ffee::m::test_tuple_range_and_literal --args 5u64 99u64

//# run 0xc0ffee::m::test_tuple_range_and_literal --args 50u64 42u64

//# run 0xc0ffee::m::test_tuple_range_and_wildcard --args 50u64 999u64

//# run 0xc0ffee::m::test_tuple_range_and_wildcard --args 200u64 999u64
