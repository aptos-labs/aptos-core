// Comparisons of non-u64 integers and bools against a constant, returned as a value or used
// as a branch condition. The golden pins how each comparison lowers. Each function runs
// with both outcomes.

// RUN: publish --print(stackless,micro-ops)
module 0x42::cmp_imm {
    public fun u8_neq(value: u8): bool { value != 200 }
    public fun i64_lt(value: i64): bool { value < -5 }
    public fun bool_eq_true(flag: bool): bool { flag == true }
    public fun bool_neq_true(flag: bool): bool { flag != true }

    public fun i8_lt_branch(value: i8): u64 { if (value < -100) 1 else 2 }
    public fun u16_neq_branch(value: u16): u64 { if (value != 65535) 1 else 2 }
    public fun u128_ge_branch(value: u128): u64 {
        if (value >= 18446744073709551616) 1 else 2
    }
    public fun bool_eq_false_branch(flag: bool): u64 { if (flag == false) 1 else 2 }
}

// RUN: execute 0x42::cmp_imm::u8_neq --args 200
// CHECK: results: false
// RUN: execute 0x42::cmp_imm::u8_neq --args 199
// CHECK: results: true
// RUN: execute 0x42::cmp_imm::u8_neq --args 255
// CHECK: results: true

// RUN: execute 0x42::cmp_imm::i64_lt --args -6
// CHECK: results: true
// RUN: execute 0x42::cmp_imm::i64_lt --args -5
// CHECK: results: false
// RUN: execute 0x42::cmp_imm::i64_lt --args 5
// CHECK: results: false

// RUN: execute 0x42::cmp_imm::bool_eq_true --args true
// CHECK: results: true
// RUN: execute 0x42::cmp_imm::bool_eq_true --args false
// CHECK: results: false
// RUN: execute 0x42::cmp_imm::bool_neq_true --args true
// CHECK: results: false
// RUN: execute 0x42::cmp_imm::bool_neq_true --args false
// CHECK: results: true

// RUN: execute 0x42::cmp_imm::i8_lt_branch --args -101
// CHECK: results: 1
// RUN: execute 0x42::cmp_imm::i8_lt_branch --args 100
// CHECK: results: 2

// RUN: execute 0x42::cmp_imm::u16_neq_branch --args 65535
// CHECK: results: 2
// RUN: execute 0x42::cmp_imm::u16_neq_branch --args 65534
// CHECK: results: 1
// Same low byte as 65535.
// RUN: execute 0x42::cmp_imm::u16_neq_branch --args 255
// CHECK: results: 1

// RUN: execute 0x42::cmp_imm::u128_ge_branch --args 18446744073709551616
// CHECK: results: 1
// RUN: execute 0x42::cmp_imm::u128_ge_branch --args 18446744073709551615
// CHECK: results: 2

// RUN: execute 0x42::cmp_imm::bool_eq_false_branch --args false
// CHECK: results: 1
// RUN: execute 0x42::cmp_imm::bool_eq_false_branch --args true
// CHECK: results: 2
