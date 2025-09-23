module 0x42::not_supported_ops {

    fun test_or1(x: i8, y: i8): i8 {
        x | y
    }

    fun test_or2(x: i16, y: i16): i16 {
        x | y
    }

    fun test_or3(x: i32, y: i32): i32 {
        x | y
    }

    fun test_or4(x: i64, y: i64): i64 {
        x | y
    }

    fun test_or5(x: i128, y: i128): i128 {
        x | y
    }

    fun test_or6(x: i256, y: i256): i256 {
        x | y
    }

    fun test_and1(x: i8, y: i8): i64 {
        x & y
    }

    fun test_and2(x: i16, y: i16): i128 {
        x & y
    }

    fun test_and3(x: i32, y: i32): i64 {
        x & y
    }

    fun test_and4(x: i64, y: i64): i128 {
        x & y
    }

    fun test_and5(x: i128, y: i128): i64 {
        x & y
    }

    fun test_and6(x: i256, y: i128): i128 {
        x & y
    }

    fun test_xor1(x: i8, y: i8): i8 {
        x ^ y
    }

    fun test_xor2(x: i16, y: i16): i16 {
        x ^ y
    }

    fun test_xor3(x: i32, y: i32): i32 {
        x ^ y
    }

    fun test_xor4(x: i64, y: i64): i64 {
        x ^ y
    }

    fun test_xor5(x: i128, y: i128): i128 {
        x ^ y
    }

    fun test_xor6(x: i256, y: i256): i256 {
        x ^ y
    }

    fun test_lsf1(x: i8, y: u8): i64 {
        x << y
    }

    fun test_lsf2(x: i16, y: u8): i128 {
        x << y
    }

    fun test_lsf3(x: i32, y: u8): i64 {
        x << y
    }

    fun test_lsf4(x: i64, y: u8): i128 {
        x << y
    }

    fun test_lsf5(x: i128, y: u8): i64 {
        x << y
    }

    fun test_lsf6(x: i256, y: u8): i128 {
        x << y
    }

    fun test_rsf1(x: i8, y: u8): i64 {
        x >> y
    }

    fun test_rsf2(x: i16, y: u8): i128 {
        x >> y
    }

    fun test_rsf3(x: i32, y: u8): i64 {
        x >> y
    }

    fun test_rsf4(x: i64, y: u8): i128 {
        x >> y
    }

    fun test_rsf5(x: i128, y: u8): i64 {
        x >> y
    }

    fun test_rsf6(x: i256, y: u8): i128 {
        x >> y
    }
}
