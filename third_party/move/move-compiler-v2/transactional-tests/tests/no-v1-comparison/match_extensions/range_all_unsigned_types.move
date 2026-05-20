//# publish
module 0xc0ffee::m {
    public fun test_u8(x: u8): u8 {
        match (x) {
            0..10 => 1,
            10..=100 => 2,
            _ => 3,
        }
    }

    public fun test_u16(x: u16): u16 {
        match (x) {
            0..1000 => 1,
            1000..=5000 => 2,
            _ => 3,
        }
    }

    public fun test_u32(x: u32): u32 {
        match (x) {
            0..1000 => 1,
            1000..=5000 => 2,
            _ => 3,
        }
    }

    public fun test_u64(x: u64): u64 {
        match (x) {
            0..1000 => 1,
            1000..=5000 => 2,
            _ => 3,
        }
    }

    public fun test_u128(x: u128): u128 {
        match (x) {
            0..1000 => 1,
            1000..=5000 => 2,
            _ => 3,
        }
    }

    public fun test_u256(x: u256): u256 {
        match (x) {
            0..1000 => 1,
            1000..=5000 => 2,
            _ => 3,
        }
    }
}

//# run 0xc0ffee::m::test_u8 --args 0u8

//# run 0xc0ffee::m::test_u8 --args 5u8

//# run 0xc0ffee::m::test_u8 --args 10u8

//# run 0xc0ffee::m::test_u8 --args 50u8

//# run 0xc0ffee::m::test_u8 --args 200u8

//# run 0xc0ffee::m::test_u16 --args 0u16

//# run 0xc0ffee::m::test_u16 --args 500u16

//# run 0xc0ffee::m::test_u16 --args 1000u16

//# run 0xc0ffee::m::test_u16 --args 3000u16

//# run 0xc0ffee::m::test_u16 --args 10000u16

//# run 0xc0ffee::m::test_u32 --args 0u32

//# run 0xc0ffee::m::test_u32 --args 1000u32

//# run 0xc0ffee::m::test_u32 --args 5000u32

//# run 0xc0ffee::m::test_u32 --args 10000u32

//# run 0xc0ffee::m::test_u64 --args 0u64

//# run 0xc0ffee::m::test_u64 --args 1000u64

//# run 0xc0ffee::m::test_u64 --args 5000u64

//# run 0xc0ffee::m::test_u64 --args 10000u64

//# run 0xc0ffee::m::test_u128 --args 0u128

//# run 0xc0ffee::m::test_u128 --args 1000u128

//# run 0xc0ffee::m::test_u128 --args 5000u128

//# run 0xc0ffee::m::test_u256 --args 0u256

//# run 0xc0ffee::m::test_u256 --args 1000u256

//# run 0xc0ffee::m::test_u256 --args 5000u256
