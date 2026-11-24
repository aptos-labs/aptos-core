// u64 bitwise ops against a constant, and ordering comparisons whose result is returned as
// a value rather than used as a branch condition.

// RUN: publish --print(micro-ops)
module 0x1::u64_bitwise_imm_and_cmp_values {
    fun and_imm(value: u64): u64 { value & 0xff }
    fun or_imm(value: u64): u64 { value | 1 }
    fun xor_imm(value: u64): u64 { value ^ 0xffff }
    fun gt(lhs: u64, rhs: u64): bool { lhs > rhs }
    fun le(lhs: u64, rhs: u64): bool { lhs <= rhs }
    fun ge(lhs: u64, rhs: u64): bool { lhs >= rhs }
    fun gt_imm(value: u64): bool { value > 10 }
    fun le_imm(value: u64): bool { value <= 10 }
    fun ge_imm(value: u64): bool { value >= 10 }
    fun lt_imm(value: u64): bool { value < 10 }
}

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::and_imm --args 4660
// CHECK: results: 52

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::and_imm --args 18446744073709551615
// CHECK: results: 255

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::or_imm --args 0
// CHECK: results: 1

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::or_imm --args 18446744073709551614
// CHECK: results: 18446744073709551615

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::xor_imm --args 255
// CHECK: results: 65280

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::xor_imm --args 18446744073709551615
// CHECK: results: 18446744073709486080

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt --args 5, 5
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt --args 4, 5
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt --args 6, 5
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt --args 0, 18446744073709551615
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le --args 5, 5
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le --args 4, 5
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le --args 6, 5
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le --args 0, 18446744073709551615
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge --args 5, 5
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge --args 4, 5
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge --args 6, 5
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge --args 0, 18446744073709551615
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt_imm --args 9
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt_imm --args 10
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::gt_imm --args 11
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le_imm --args 9
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le_imm --args 10
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::le_imm --args 11
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge_imm --args 9
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge_imm --args 10
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::ge_imm --args 11
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::lt_imm --args 9
// CHECK: results: true

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::lt_imm --args 10
// CHECK: results: false

// RUN: execute 0x1::u64_bitwise_imm_and_cmp_values::lt_imm --args 11
// CHECK: results: false
