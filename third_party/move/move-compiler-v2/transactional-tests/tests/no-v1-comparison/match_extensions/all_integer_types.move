//# publish
module 0xc0ffee::m {
    // Test all unsigned integer types
    public fun test_u8(x: u8): u8 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 99,
        }
    }

    public fun test_u16(x: u16): u16 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 999,
        }
    }

    public fun test_u32(x: u32): u32 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 999,
        }
    }

    public fun test_u64(x: u64): u64 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 999,
        }
    }

    public fun test_u128(x: u128): u128 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 999,
        }
    }

    public fun test_u256(x: u256): u256 {
        match (x) {
            0 => 0,
            1 => 1,
            _ => 999,
        }
    }

    // Test all signed integer types
    public fun test_i8(x: i8): i8 {
        match (x) {
            -1 => -1,
            0 => 0,
            _ => 7,
        }
    }

    public fun test_i16(x: i16): i16 {
        match (x) {
            -1 => -1,
            0 => 0,
            _ => 7,
        }
    }

    public fun test_i32(x: i32): i32 {
        match (x) {
            -1 => -1,
            0 => 0,
            _ => 7,
        }
    }

    public fun test_i64(x: i64): i64 {
        match (x) {
            -1 => -1,
            0 => 0,
            _ => 7,
        }
    }

    public fun test_i128(x: i128): i128 {
        match (x) {
            -1 => -1,
            0 => 0,
            _ => 7,
        }
    }

    public fun test_i256(x: i256): i256 {
        match ((x, x)) {
            (-1, -1) => -1,
            (0, 0) => 0,
            _ => 7,
        }
    }
}

//# run 0xc0ffee::m::test_u8 --args 0u8

//# run 0xc0ffee::m::test_u8 --args 1u8

//# run 0xc0ffee::m::test_u8 --args 50u8

//# run 0xc0ffee::m::test_u16 --args 0u16

//# run 0xc0ffee::m::test_u16 --args 1u16

//# run 0xc0ffee::m::test_u16 --args 500u16

//# run 0xc0ffee::m::test_u32 --args 0u32

//# run 0xc0ffee::m::test_u32 --args 1u32

//# run 0xc0ffee::m::test_u32 --args 500u32

//# run 0xc0ffee::m::test_u64 --args 0u64

//# run 0xc0ffee::m::test_u64 --args 1u64

//# run 0xc0ffee::m::test_u64 --args 500u64

//# run 0xc0ffee::m::test_u128 --args 0u128

//# run 0xc0ffee::m::test_u128 --args 1u128

//# run 0xc0ffee::m::test_u256 --args 0u256

//# run 0xc0ffee::m::test_u256 --args 1u256

//# run 0xc0ffee::m::test_i8 --args -1i8

//# run 0xc0ffee::m::test_i8 --args 0i8

//# run 0xc0ffee::m::test_i8 --args 5i8

//# run 0xc0ffee::m::test_i16 --args -1i16

//# run 0xc0ffee::m::test_i16 --args 0i16

//# run 0xc0ffee::m::test_i16 --args 5i16

//# run 0xc0ffee::m::test_i32 --args -1i32

//# run 0xc0ffee::m::test_i32 --args 0i32

//# run 0xc0ffee::m::test_i32 --args 5i32

//# run 0xc0ffee::m::test_i64 --args -1i64

//# run 0xc0ffee::m::test_i64 --args 0i64

//# run 0xc0ffee::m::test_i64 --args 5i64

//# run 0xc0ffee::m::test_i128 --args -1i128

//# run 0xc0ffee::m::test_i128 --args 0i128

//# run 0xc0ffee::m::test_i128 --args 5i128

//# run 0xc0ffee::m::test_i256 --args -1i256

//# run 0xc0ffee::m::test_i256 --args 0i256

//# run 0xc0ffee::m::test_i256 --args 5i256
