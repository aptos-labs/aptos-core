//# publish
module 0xc0ffee::m {
    // Ranges crossing zero
    public fun test_i8_cross_zero(x: i8): u64 {
        match (x) {
            -5..5 => 1,
            _ => 0,
        }
    }

    // Negative ranges
    public fun test_i8_negative(x: i8): u64 {
        match (x) {
            -100..-50 => 1,
            _ => 0,
        }
    }

    public fun test_i16(x: i16): u64 {
        match (x) {
            -1000..0 => 1,
            0..=1000 => 2,
            _ => 0,
        }
    }

    public fun test_i32(x: i32): u64 {
        match (x) {
            -100..0 => 1,
            0..=100 => 2,
            _ => 0,
        }
    }

    public fun test_i64(x: i64): u64 {
        match (x) {
            -100..0 => 1,
            0..=100 => 2,
            _ => 0,
        }
    }

    public fun test_i128(x: i128): u64 {
        match (x) {
            -100..0 => 1,
            0..=100 => 2,
            _ => 0,
        }
    }

    public fun test_i256(x: i256): u64 {
        match (x) {
            -100..0 => 1,
            0..=100 => 2,
            _ => 0,
        }
    }
}

//# run 0xc0ffee::m::test_i8_cross_zero --args -5i8

//# run 0xc0ffee::m::test_i8_cross_zero --args 0i8

//# run 0xc0ffee::m::test_i8_cross_zero --args 4i8

//# run 0xc0ffee::m::test_i8_cross_zero --args 5i8

//# run 0xc0ffee::m::test_i8_negative --args -100i8

//# run 0xc0ffee::m::test_i8_negative --args -75i8

//# run 0xc0ffee::m::test_i8_negative --args -50i8

//# run 0xc0ffee::m::test_i8_negative --args -49i8

//# run 0xc0ffee::m::test_i16 --args -500i16

//# run 0xc0ffee::m::test_i16 --args 0i16

//# run 0xc0ffee::m::test_i16 --args 500i16

//# run 0xc0ffee::m::test_i16 --args 2000i16

//# run 0xc0ffee::m::test_i32 --args -50i32

//# run 0xc0ffee::m::test_i32 --args 0i32

//# run 0xc0ffee::m::test_i32 --args 50i32

//# run 0xc0ffee::m::test_i32 --args 200i32

//# run 0xc0ffee::m::test_i64 --args -50i64

//# run 0xc0ffee::m::test_i64 --args 0i64

//# run 0xc0ffee::m::test_i64 --args 50i64

//# run 0xc0ffee::m::test_i128 --args -50i128

//# run 0xc0ffee::m::test_i128 --args 0i128

//# run 0xc0ffee::m::test_i128 --args 50i128

//# run 0xc0ffee::m::test_i256 --args -50i256

//# run 0xc0ffee::m::test_i256 --args 0i256

//# run 0xc0ffee::m::test_i256 --args 50i256
