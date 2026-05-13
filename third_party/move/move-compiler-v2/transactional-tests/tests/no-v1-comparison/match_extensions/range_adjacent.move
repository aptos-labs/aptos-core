//# publish
module 0xc0ffee::m {
    // Adjacent ranges: no gap, first-match semantics
    public fun test_adjacent(x: u64): u64 {
        match (x) {
            0..5 => 1,
            5..10 => 2,
            _ => 3,
        }
    }

    // Overlapping ranges: first match wins
    public fun test_overlapping(x: u64): u64 {
        match (x) {
            0..10 => 1,
            5..15 => 2,
            _ => 3,
        }
    }

    // Exact coverage with exclusive ranges
    public fun test_exact_coverage(x: u8): u64 {
        match (x) {
            0..64 => 1,
            64..128 => 2,
            128..192 => 3,
            192.. => 4,
        }
    }
}

//# run 0xc0ffee::m::test_adjacent --args 0u64

//# run 0xc0ffee::m::test_adjacent --args 4u64

//# run 0xc0ffee::m::test_adjacent --args 5u64

//# run 0xc0ffee::m::test_adjacent --args 9u64

//# run 0xc0ffee::m::test_adjacent --args 10u64

//# run 0xc0ffee::m::test_overlapping --args 0u64

//# run 0xc0ffee::m::test_overlapping --args 5u64

//# run 0xc0ffee::m::test_overlapping --args 7u64

//# run 0xc0ffee::m::test_overlapping --args 10u64

//# run 0xc0ffee::m::test_overlapping --args 14u64

//# run 0xc0ffee::m::test_overlapping --args 15u64

//# run 0xc0ffee::m::test_exact_coverage --args 0u8

//# run 0xc0ffee::m::test_exact_coverage --args 63u8

//# run 0xc0ffee::m::test_exact_coverage --args 64u8

//# run 0xc0ffee::m::test_exact_coverage --args 127u8

//# run 0xc0ffee::m::test_exact_coverage --args 128u8

//# run 0xc0ffee::m::test_exact_coverage --args 191u8

//# run 0xc0ffee::m::test_exact_coverage --args 192u8

//# run 0xc0ffee::m::test_exact_coverage --args 255u8
