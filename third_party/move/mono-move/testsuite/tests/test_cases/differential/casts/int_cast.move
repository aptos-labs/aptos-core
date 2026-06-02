// RUN: publish
module 0x1::test {
    // Widening, same sign — always succeeds.
    fun u64_to_u128(x: u64): u128 { x as u128 }
    fun i64_to_i256(x: i64): i256 { x as i256 }

    // Narrowing, same sign — aborts when the value doesn't fit.
    fun u128_to_u64(x: u128): u64 { x as u64 }
    fun u256_to_u128(x: u256): u128 { x as u128 }
    fun i128_to_i64(x: i128): i64 { x as i64 }

    // Unsigned -> signed.
    fun u64_to_i64(x: u64): i64 { x as i64 }      // same width: top half overflows
    fun u64_to_i128(x: u64): i128 { x as i128 }   // widening: always fits
    fun u256_to_i256(x: u256): i256 { x as i256 } // same width: top half overflows
    fun u256_to_i64(x: u256): i64 { x as i64 }    // narrowing

    // Signed -> unsigned — aborts on negative inputs.
    fun i64_to_u64(x: i64): u64 { x as u64 }
    fun i64_to_u128(x: i64): u128 { x as u128 }   // widening: negative still aborts
    fun i256_to_u256(x: i256): u256 { x as u256 }
}

// In-range casts — both VMs return the same value.

// RUN: execute 0x1::test::u64_to_u128 --args 12345
// CHECK: results: 12345

// RUN: execute 0x1::test::i64_to_i256 --args -42
// CHECK: results: -42

// RUN: execute 0x1::test::u128_to_u64 --args 255
// CHECK: results: 255

// u128::MAX-shaped value that still fits u64 (== u64::MAX).
// RUN: execute 0x1::test::u128_to_u64 --args 18446744073709551615
// CHECK: results: 18446744073709551615

// RUN: execute 0x1::test::u256_to_u128 --args 1000000000000000000000
// CHECK: results: 1000000000000000000000

// RUN: execute 0x1::test::i128_to_i64 --args -1000
// CHECK: results: -1000

// u64 value below i64::MAX casts cleanly.
// RUN: execute 0x1::test::u64_to_i64 --args 5
// CHECK: results: 5

// u64::MAX widens into i128 without loss.
// RUN: execute 0x1::test::u64_to_i128 --args 18446744073709551615
// CHECK: results: 18446744073709551615

// RUN: execute 0x1::test::u256_to_i256 --args 100
// CHECK: results: 100

// Below i64::MAX (9223372036854775807).
// RUN: execute 0x1::test::u256_to_i64 --args 9000000000000000000
// CHECK: results: 9000000000000000000

// RUN: execute 0x1::test::i64_to_u64 --args 7
// CHECK: results: 7

// RUN: execute 0x1::test::i64_to_u128 --args 7
// CHECK: results: 7

// RUN: execute 0x1::test::i256_to_u256 --args 123456789
// CHECK: results: 123456789

// Out-of-range casts — both VMs abort.

// 2^64 doesn't fit u64.
// RUN: execute 0x1::test::u128_to_u64 --args 18446744073709551616
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// u256 value above u128::MAX.
// RUN: execute 0x1::test::u256_to_u128 --args 340282366920938463463374607431768211456
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// i64::MAX + 1 doesn't fit i64.
// RUN: execute 0x1::test::i128_to_i64 --args 9223372036854775808
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// u64::MAX > i64::MAX.
// RUN: execute 0x1::test::u64_to_i64 --args 18446744073709551615
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// u256::MAX > i256::MAX (top half of the unsigned range).
// RUN: execute 0x1::test::u256_to_i256 --args 115792089237316195423570985008687907853269984665640564039457584007913129639935
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// Negative -> unsigned aborts.
// RUN: execute 0x1::test::i64_to_u64 --args -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// Negative -> wider unsigned still aborts.
// RUN: execute 0x1::test::i64_to_u128 --args -5
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range

// RUN: execute 0x1::test::i256_to_u256 --args -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: value out of range
