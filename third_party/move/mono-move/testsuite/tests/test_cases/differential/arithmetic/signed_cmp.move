// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun i64_lt(a: i64, b: i64): u64 {
        if (a < b) { 1 } else { 2 }
    }

    fun i64_ge(a: i64, b: i64): u64 {
        if (a >= b) { 1 } else { 2 }
    }

    fun i8_lt(a: i8, b: i8): u64 {
        if (a < b) { 1 } else { 2 }
    }

    fun i128_lt(a: i128, b: i128): u64 {
        if (a < b) { 1 } else { 2 }
    }
}

// RUN: execute 0x1::test::i64_lt --args -1, 1
// CHECK: results: 1
// RUN: execute 0x1::test::i64_lt --args 1, -1
// CHECK: results: 2
// RUN: execute 0x1::test::i64_lt --args -5, -2
// CHECK: results: 1

// RUN: execute 0x1::test::i64_ge --args -1, -1
// CHECK: results: 1
// RUN: execute 0x1::test::i64_ge --args -2, 3
// CHECK: results: 2

// RUN: execute 0x1::test::i8_lt --args -1, 1
// CHECK: results: 1
// RUN: execute 0x1::test::i128_lt --args -1, 1
// CHECK: results: 1
