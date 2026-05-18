//# publish
module 0xc0ffee::m {
    // Ranges and exact literal patterns in the same match
    public fun test_mixed(x: u64): u64 {
        match (x) {
            0 => 1,
            1..10 => 2,
            10 => 3,
            11..100 => 4,
            _ => 5,
        }
    }

    // Literal before and after range
    public fun test_literal_sandwich(x: u8): u64 {
        match (x) {
            0 => 1,
            1..=254 => 2,
            255 => 3,
        }
    }
}

//# run 0xc0ffee::m::test_mixed --args 0u64

//# run 0xc0ffee::m::test_mixed --args 1u64

//# run 0xc0ffee::m::test_mixed --args 5u64

//# run 0xc0ffee::m::test_mixed --args 9u64

//# run 0xc0ffee::m::test_mixed --args 10u64

//# run 0xc0ffee::m::test_mixed --args 50u64

//# run 0xc0ffee::m::test_mixed --args 100u64

//# run 0xc0ffee::m::test_literal_sandwich --args 0u8

//# run 0xc0ffee::m::test_literal_sandwich --args 128u8

//# run 0xc0ffee::m::test_literal_sandwich --args 255u8
