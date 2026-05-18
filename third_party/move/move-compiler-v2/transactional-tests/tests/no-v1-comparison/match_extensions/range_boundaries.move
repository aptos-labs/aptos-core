//# publish
module 0xc0ffee::m {
    // 0u8..255u8 covers all but 255
    public fun test_u8_almost_full(x: u8): u64 {
        match (x) {
            0..255 => 1,
            _ => 2,
        }
    }

    // 0u8..=255u8 covers all u8 values
    public fun test_u8_full_inclusive(x: u8): u64 {
        match (x) {
            0..=255 => 1,
        }
    }

    // Single value exclusive range: 0u8..1u8 matches only 0
    public fun test_u8_single_exclusive(x: u8): u64 {
        match (x) {
            0..1 => 1,
            _ => 0,
        }
    }

    // Single value inclusive range: 5u8..=5u8 matches only 5
    public fun test_u8_single_inclusive(x: u8): u64 {
        match (x) {
            5..=5 => 1,
            _ => 0,
        }
    }

    // Open range near MAX: 200u8.. covers 200-255
    public fun test_u8_open_near_max(x: u8): u64 {
        match (x) {
            200.. => 1,
            _ => 0,
        }
    }

    // u64 MAX boundary
    public fun test_u64_max(x: u64): u64 {
        match (x) {
            0..18446744073709551615 => 1,
            _ => 2,
        }
    }
}

//# run 0xc0ffee::m::test_u8_almost_full --args 0u8

//# run 0xc0ffee::m::test_u8_almost_full --args 128u8

//# run 0xc0ffee::m::test_u8_almost_full --args 254u8

//# run 0xc0ffee::m::test_u8_almost_full --args 255u8

//# run 0xc0ffee::m::test_u8_full_inclusive --args 0u8

//# run 0xc0ffee::m::test_u8_full_inclusive --args 128u8

//# run 0xc0ffee::m::test_u8_full_inclusive --args 255u8

//# run 0xc0ffee::m::test_u8_single_exclusive --args 0u8

//# run 0xc0ffee::m::test_u8_single_exclusive --args 1u8

//# run 0xc0ffee::m::test_u8_single_inclusive --args 4u8

//# run 0xc0ffee::m::test_u8_single_inclusive --args 5u8

//# run 0xc0ffee::m::test_u8_single_inclusive --args 6u8

//# run 0xc0ffee::m::test_u8_open_near_max --args 199u8

//# run 0xc0ffee::m::test_u8_open_near_max --args 200u8

//# run 0xc0ffee::m::test_u8_open_near_max --args 255u8

//# run 0xc0ffee::m::test_u64_max --args 0u64

//# run 0xc0ffee::m::test_u64_max --args 18446744073709551614u64

//# run 0xc0ffee::m::test_u64_max --args 18446744073709551615u64
