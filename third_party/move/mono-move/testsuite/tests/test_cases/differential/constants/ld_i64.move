// Returning i64 literals (a negative value, the minimum and the maximum) with the sign
// intact. The golden pins how the literals lower.

// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun neg(): i64 { -5 }
    fun min_val(): i64 { -9223372036854775808 }
    fun max_val(): i64 { 9223372036854775807 }
}

// RUN: execute 0x1::test::neg
// CHECK: results: -5

// RUN: execute 0x1::test::min_val
// CHECK: results: -9223372036854775808

// RUN: execute 0x1::test::max_val
// CHECK: results: 9223372036854775807
