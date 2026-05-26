//# publish
module 0xff::shift_max {
    public fun u8_max():   u8   { 255u8 }
    public fun u16_max():  u16  { 65535u16 }
    public fun u32_max():  u32  { 4294967295u32 }
    public fun u64_max():  u64  { 18446744073709551615u64 }
    public fun u128_max(): u128 { 340282366920938463463374607431768211455u128 }
    public fun u256_max(): u256 {
        115792089237316195423570985008687907853269984665640564039457584007913129639935u256
    }

    public fun u8_max_shl1():   u8   { u8_max()   << 1u8 }
    public fun u16_max_shl1():  u16  { u16_max()  << 1u8 }
    public fun u32_max_shl1():  u32  { u32_max()  << 1u8 }
    public fun u64_max_shl1():  u64  { u64_max()  << 1u8 }
    public fun u128_max_shl1(): u128 { u128_max() << 1u8 }
    public fun u256_max_shl1(): u256 { u256_max() << 1u8 }
}

//# run --verbose
script {
    fun main() {
        assert!(255u8 << 1u8 == 254u8, 100);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(65535u16 << 1u8 == 65534u16, 200);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(4294967295u32 << 1u8 == 4294967294u32, 300);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(18446744073709551615u64 << 1u8 == 18446744073709551614u64, 400);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(
            340282366920938463463374607431768211455u128 << 1u8
                == 340282366920938463463374607431768211454u128,
            500,
        );
    }
}

//# run --verbose
script {
    fun main() {
        assert!(
            115792089237316195423570985008687907853269984665640564039457584007913129639935u256 << 1u8
                == 115792089237316195423570985008687907853269984665640564039457584007913129639934u256,
            600,
        );
    }
}

//# run 0xff::shift_max::u8_max_shl1 --verbose

//# run 0xff::shift_max::u16_max_shl1 --verbose

//# run 0xff::shift_max::u32_max_shl1 --verbose

//# run 0xff::shift_max::u64_max_shl1 --verbose

//# run 0xff::shift_max::u128_max_shl1 --verbose

//# run 0xff::shift_max::u256_max_shl1 --verbose

//# publish
module 0xff::const_shift_max {
    const C_U8:   u8   = 255u8 << 1u8;
    const C_U16:  u16  = 65535u16 << 1u8;
    const C_U32:  u32  = 4294967295u32 << 1u8;
    const C_U64:  u64  = 18446744073709551615u64 << 1u8;
    const C_U128: u128 =
        340282366920938463463374607431768211455u128 << 1u8;
    const C_U256: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935u256 << 1u8;

    public fun c_u8():   u8   { C_U8 }
    public fun c_u16():  u16  { C_U16 }
    public fun c_u32():  u32  { C_U32 }
    public fun c_u64():  u64  { C_U64 }
    public fun c_u128(): u128 { C_U128 }
    public fun c_u256(): u256 { C_U256 }
}

//# run 0xff::const_shift_max::c_u8 --verbose

//# run 0xff::const_shift_max::c_u16 --verbose

//# run 0xff::const_shift_max::c_u32 --verbose

//# run 0xff::const_shift_max::c_u64 --verbose

//# run 0xff::const_shift_max::c_u128 --verbose

//# run 0xff::const_shift_max::c_u256 --verbose
