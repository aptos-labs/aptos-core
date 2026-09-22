// Differential coverage for wide signed arithmetic on the unspecialized
// integer path: i128 division and remainder with negative operands and at
// i128::MIN, and i256 multiply/divide.

// RUN: publish
module 0x1::test {
    fun i128_div_min(a: i128): i128 { a / 1 }
    fun i128_mod(a: i128, b: i128): i128 { a % b }

    fun i256_mul(a: i256, b: i256): i256 { a * b }
    fun i256_div(a: i256, b: i256): i256 { a / b }
}

// i128::MIN / 1 is MIN exactly (no overflow).
// RUN: execute 0x1::test::i128_div_min --args -170141183460469231731687303715884105728
// CHECK: results: -170141183460469231731687303715884105728

// Signed remainder keeps the dividend's sign.
// RUN: execute 0x1::test::i128_mod --args -99999999999999999999, 7
// CHECK: results: -1

// RUN: execute 0x1::test::i256_mul --args -2, 3
// CHECK: results: -6

// RUN: execute 0x1::test::i256_div --args -1000000000000000000000, 3
// CHECK: results: -333333333333333333333
