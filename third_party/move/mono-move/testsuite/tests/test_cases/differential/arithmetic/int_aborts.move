// Differential coverage for the abort paths in the unspecialized
// integer micro-ops: overflow / underflow on signed and unsigned widths,
// division and modulus by zero, and signed-negate of MIN.
//
// V1 and V2 emit very different abort messages, and the exact phrasing
// of each is not load-bearing for this test — we only care that *both*
// VMs abort. So we substring-check `ARITHMETIC_ERROR` (V1) and `error:`
// (V2) and leave the rest of the message floating.

// RUN: publish
module 0x1::test {
    fun u128_add(a: u128, b: u128): u128 { a + b }
    fun u128_sub(a: u128, b: u128): u128 { a - b }
    fun u128_mul(a: u128, b: u128): u128 { a * b }
    fun u128_div(a: u128, b: u128): u128 { a / b }
    fun u128_mod(a: u128, b: u128): u128 { a % b }

    fun u256_add(a: u256, b: u256): u256 { a + b }

    fun i64_add(a: i64, b: i64): i64 { a + b }
    fun i64_sub(a: i64, b: i64): i64 { a - b }
    fun i64_mul(a: i64, b: i64): i64 { a * b }
    fun i64_div(a: i64, b: i64): i64 { a / b }
    fun i64_neg(a: i64): i64 { -a }

    fun i128_add(a: i128, b: i128): i128 { a + b }
    fun i128_neg(a: i128): i128 { -a }
    fun i256_neg(a: i256): i256 { -a }
}

// Unsigned overflow / underflow.

// RUN: execute 0x1::test::u128_add --args 340282366920938463463374607431768211455, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::u128_sub --args 0, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::u128_mul --args 18446744073709551616, 18446744073709551616
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::u256_add --args 115792089237316195423570985008687907853269984665640564039457584007913129639935, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// Division / modulus by zero.

// RUN: execute 0x1::test::u128_div --args 100, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::u128_mod --args 100, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i64_div --args 100, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// Signed overflow / underflow.

// RUN: execute 0x1::test::i64_add --args 9223372036854775807, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i64_sub --args -9223372036854775808, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i64_mul --args 9223372036854775807, 2
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i128_add --args 170141183460469231731687303715884105727, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// `i_N::MIN / -1` overflows: the absolute value of MIN cannot be
// represented as a positive i_N.
// RUN: execute 0x1::test::i64_div --args -9223372036854775808, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// Signed `-MIN` overflows because the result doesn't fit in i_N.

// RUN: execute 0x1::test::i64_neg --args -9223372036854775808
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i128_neg --args -170141183460469231731687303715884105728
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:

// RUN: execute 0x1::test::i256_neg --args -57896044618658097711785492504343953926634992332820282019728792003956564819968
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:
