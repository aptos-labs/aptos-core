// Literal comparisons exercise width and signedness handling in `JumpIntCmp`
// immediates. Signed division and remainder cover truncation toward zero.

// RUN: publish
module 0x1::test {
    fun i8_lt_neg100(x: i8): u64 { if (x < -100) { 1 } else { 2 } }
    fun i8_ge_min(x: i8): u64 { if (x >= -128) { 1 } else { 2 } }
    fun i8_eq_min(x: i8): u64 { if (x == -128) { 1 } else { 2 } }

    fun i16_lt_neg300(x: i16): u64 { if (x < -300) { 1 } else { 2 } }
    fun i16_eq_min(x: i16): u64 { if (x == -32768) { 1 } else { 2 } }

    fun i32_lt_imm(x: i32): u64 { if (x < -2000000000) { 1 } else { 2 } }
    fun i32_eq_min(x: i32): u64 { if (x == -2147483648) { 1 } else { 2 } }

    fun i64_lt_big(x: i64): u64 { if (x < -9223372036854775807) { 1 } else { 2 } }

    fun i128_lt_imm(x: i128): u64 {
        if (x < -170141183460469231731687303715884105727) { 1 } else { 2 }
    }

    fun i256_lt_neg1(x: i256): u64 { if (x < -1) { 1 } else { 2 } }

    fun u8_eq_max(x: u8): u64 { if (x == 255) { 1 } else { 2 } }
    fun u16_ge_max(x: u16): u64 { if (x >= 65535) { 1 } else { 2 } }
    fun u32_lt_imm(x: u32): u64 { if (x < 65536) { 1 } else { 2 } }
    fun u64_eq_max(x: u64): u64 { if (x == 18446744073709551615) { 1 } else { 2 } }
    fun u128_ge_max(x: u128): u64 {
        if (x >= 340282366920938463463374607431768211455) { 1 } else { 2 }
    }

    // Signed remainder keeps the dividend's sign (truncating division).
    fun i8_mod(a: i8, b: i8): i8 { a % b }
    fun i16_div(a: i16, b: i16): i16 { a / b }
}

// i8 against negative immediates.
// RUN: execute 0x1::test::i8_lt_neg100 --args -128
// CHECK: results: 1
// RUN: execute 0x1::test::i8_lt_neg100 --args -100
// CHECK: results: 2
// RUN: execute 0x1::test::i8_lt_neg100 --args -99
// CHECK: results: 2
// RUN: execute 0x1::test::i8_lt_neg100 --args 127
// CHECK: results: 2
// RUN: execute 0x1::test::i8_ge_min --args -128
// CHECK: results: 1
// RUN: execute 0x1::test::i8_ge_min --args 127
// CHECK: results: 1
// RUN: execute 0x1::test::i8_eq_min --args -128
// CHECK: results: 1
// RUN: execute 0x1::test::i8_eq_min --args 0
// CHECK: results: 2

// i16 / i32 immediates.
// RUN: execute 0x1::test::i16_lt_neg300 --args -32768
// CHECK: results: 1
// RUN: execute 0x1::test::i16_lt_neg300 --args -300
// CHECK: results: 2
// RUN: execute 0x1::test::i16_lt_neg300 --args 300
// CHECK: results: 2
// RUN: execute 0x1::test::i16_eq_min --args -32768
// CHECK: results: 1
// RUN: execute 0x1::test::i32_lt_imm --args -2147483648
// CHECK: results: 1
// RUN: execute 0x1::test::i32_lt_imm --args -1
// CHECK: results: 2
// RUN: execute 0x1::test::i32_eq_min --args -2147483648
// CHECK: results: 1

// Wide signed immediates.
// RUN: execute 0x1::test::i64_lt_big --args -9223372036854775808
// CHECK: results: 1
// RUN: execute 0x1::test::i64_lt_big --args 9223372036854775807
// CHECK: results: 2
// RUN: execute 0x1::test::i128_lt_imm --args -170141183460469231731687303715884105728
// CHECK: results: 1
// RUN: execute 0x1::test::i128_lt_imm --args -1
// CHECK: results: 2
// RUN: execute 0x1::test::i256_lt_neg1 --args -2
// CHECK: results: 1
// RUN: execute 0x1::test::i256_lt_neg1 --args -1
// CHECK: results: 2
// RUN: execute 0x1::test::i256_lt_neg1 --args 0
// CHECK: results: 2

// Unsigned immediates at width boundaries.
// RUN: execute 0x1::test::u8_eq_max --args 255
// CHECK: results: 1
// RUN: execute 0x1::test::u8_eq_max --args 254
// CHECK: results: 2
// RUN: execute 0x1::test::u16_ge_max --args 65535
// CHECK: results: 1
// RUN: execute 0x1::test::u16_ge_max --args 65534
// CHECK: results: 2
// RUN: execute 0x1::test::u32_lt_imm --args 65535
// CHECK: results: 1
// RUN: execute 0x1::test::u32_lt_imm --args 65536
// CHECK: results: 2
// RUN: execute 0x1::test::u64_eq_max --args 18446744073709551615
// CHECK: results: 1
// RUN: execute 0x1::test::u64_eq_max --args 18446744073709551614
// CHECK: results: 2
// RUN: execute 0x1::test::u128_ge_max --args 340282366920938463463374607431768211455
// CHECK: results: 1
// RUN: execute 0x1::test::u128_ge_max --args 0
// CHECK: results: 2

// Signed div/mod rounding (truncation toward zero).
// RUN: execute 0x1::test::i8_mod --args -100, 7
// CHECK: results: -2
// RUN: execute 0x1::test::i8_mod --args 100, -7
// CHECK: results: 2
// RUN: execute 0x1::test::i8_mod --args -100, -7
// CHECK: results: -2
// RUN: execute 0x1::test::i16_div --args -100, 7
// CHECK: results: -14
// RUN: execute 0x1::test::i16_div --args 100, -7
// CHECK: results: -14
