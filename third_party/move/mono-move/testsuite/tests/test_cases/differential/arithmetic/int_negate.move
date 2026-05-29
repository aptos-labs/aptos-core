// Differential coverage for `IntNegate` (signed negate).

// RUN: publish
module 0x1::test {
    fun i64_neg(a: i64): i64 { -a }
    fun i128_neg(a: i128): i128 { -a }
    fun i256_neg(a: i256): i256 { -a }
}

// RUN: execute 0x1::test::i64_neg --args 1
// CHECK: results: -1

// RUN: execute 0x1::test::i64_neg --args 100
// CHECK: results: -100

// RUN: execute 0x1::test::i64_neg --args -42
// CHECK: results: 42

// RUN: execute 0x1::test::i128_neg --args 1
// CHECK: results: -1

// RUN: execute 0x1::test::i128_neg --args 42
// CHECK: results: -42

// RUN: execute 0x1::test::i256_neg --args 1234567
// CHECK: results: -1234567

// RUN: execute 0x1::test::i256_neg --args 0
// CHECK: results: 0
