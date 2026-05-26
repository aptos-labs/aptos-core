// Differential coverage for the unspecialized integer arithmetic path.
// u64 itself flows through the specialized fast path; the widths
// exercised here (u128, u256, i64, i128, i256) go through `IntAdd` etc.
// with the matching `IntOperand::Slot*` arm.

// RUN: publish
module 0x1::test {
    fun u128_add(a: u128, b: u128): u128 { a + b }
    fun u128_sub(a: u128, b: u128): u128 { a - b }
    fun u128_mul(a: u128, b: u128): u128 { a * b }
    fun u128_div(a: u128, b: u128): u128 { a / b }
    fun u128_mod(a: u128, b: u128): u128 { a % b }

    fun u256_mul(a: u256, b: u256): u256 { a * b }

    fun i64_add(a: i64, b: i64): i64 { a + b }
    fun i64_sub(a: i64, b: i64): i64 { a - b }
    fun i128_mul(a: i128, b: i128): i128 { a * b }
    fun i256_add(a: i256, b: i256): i256 { a + b }
}

// RUN: execute 0x1::test::u128_add --args 100, 50
// CHECK: results: 150

// RUN: execute 0x1::test::u128_sub --args 1000, 250
// CHECK: results: 750

// RUN: execute 0x1::test::u128_mul --args 7, 13
// CHECK: results: 91

// RUN: execute 0x1::test::u128_div --args 100, 7
// CHECK: results: 14

// RUN: execute 0x1::test::u128_mod --args 100, 7
// CHECK: results: 2

// RUN: execute 0x1::test::u256_mul --args 1000000000000, 1000000000000
// CHECK: results: 1000000000000000000000000

// RUN: execute 0x1::test::i64_add --args 30, 12
// CHECK: results: 42

// RUN: execute 0x1::test::i64_sub --args 5, 12
// CHECK: results: -7

// RUN: execute 0x1::test::i128_mul --args -6, 7
// CHECK: results: -42

// RUN: execute 0x1::test::i256_add --args 100, -250
// CHECK: results: -150

// Boundary values that do *not* overflow — exercises the
// just-under-the-edge path of each checked op.

// MAX - 1 + 1 = MAX.
// RUN: execute 0x1::test::u128_add --args 340282366920938463463374607431768211454, 1
// CHECK: results: 340282366920938463463374607431768211455

// MIN - (- (MAX - 1)) = MAX, the most negative result that still fits.
// RUN: execute 0x1::test::i64_sub --args -9223372036854775808, -1
// CHECK: results: -9223372036854775807

// MAX - 1 stays in range when produced via mul.
// RUN: execute 0x1::test::i128_mul --args -85070591730234615865843651857942052863, 2
// CHECK: results: -170141183460469231731687303715884105726
