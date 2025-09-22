module 0x42::not_supported_ops {
    fun test_or1(x: i64, y: i64): i64 {
        x | y
    }

    fun test_or2(x: i128, y: i128): i128 {
        x | y
    }

    fun test_and1(x: i64, y: i64): i64 {
        x & y
    }

    fun test_and2(x: i128, y: i128): i128 {
        x & y
    }

    fun test_xor1(x: i64, y: i64): i64 {
        x ^ y
    }

    fun test_xor2(x: i128, y: i128): i128 {
        x ^ y
    }

    fun test_lsf1(x: i64, y: u8): i64 {
        x << y
    }

    fun test_lsf2(x: i128, y: u8): i128 {
        x >> y
    }

    fun test_rsf1(x: i64, y: u8): i64 {
        x << y
    }

    fun test_rsf2(x: i128, y: u8): i128 {
        x >> y
    }

    fun test_neq1(x: i64, y: i64): bool {
        x != y
    }

    fun test_neq2(x: i128, y: i128): bool {
        x != y
    }

    fun cast1(x: i64): i128 {
        x as i128
    }

    fun cast2(x: i64): u8 {
        x as u8
    }

    fun cast3(x: i64): u16 {
        x as u16
    }

    fun cast4(x: i64): u32 {
        x as u32
    }

    fun cast5(x: i64): u64 {
        x as u64
    }

    fun cast6(x: i64): u128 {
        x as u128
    }

    fun cast7(x: i64): u256 {
        x as u256
    }

    fun cast8(x: i128): i64 {
        x as i64
    }

    fun cast9(x: i128): u8 {
        x as u8
    }

    fun cast10(x: i128): u16 {
        x as u16
    }

    fun cast11(x: i128): u32 {
        x as u32
    }

    fun cast12(x: i128): u64 {
        x as u64
    }

    fun cast13(x: i128): u128 {
        x as u128
    }

    fun cast14(x: i128): u256 {
        x as u256
    }

    fun cast15(x: u64): i128 {
        x as i128
    }

    fun cast16(x: u128): i64 {
        x as i64
    }
}
