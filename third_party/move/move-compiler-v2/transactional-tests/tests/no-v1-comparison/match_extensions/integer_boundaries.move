//# publish
module 0xc0ffee::m {
    // Test u8 maximum value
    public fun test_u8_max(x: u8): u8 {
        match (x) {
            255 => 1,  // u8::MAX
            0 => 0,
            _ => 2,
        }
    }

    // Test multiple integer types
    public fun test_u64_boundaries(x: u64): u64 {
        match (x) {
            0 => 0,
            18446744073709551615 => 1,  // u64::MAX
            _ => 2,
        }
    }

    // Test u128 boundaries
    public fun test_u128_max(x: u128): u128 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 2,
        }
    }
}

//# run 0xc0ffee::m::test_u8_max --args 255u8

//# run 0xc0ffee::m::test_u8_max --args 0u8

//# run 0xc0ffee::m::test_u8_max --args 100u8

//# run 0xc0ffee::m::test_u64_boundaries --args 0u64

//# run 0xc0ffee::m::test_u64_boundaries --args 18446744073709551615u64

//# run 0xc0ffee::m::test_u64_boundaries --args 1000u64

//# run 0xc0ffee::m::test_u128_max --args 0u128

//# run 0xc0ffee::m::test_u128_max --args 1u128

//# run 0xc0ffee::m::test_u128_max --args 1000u128
