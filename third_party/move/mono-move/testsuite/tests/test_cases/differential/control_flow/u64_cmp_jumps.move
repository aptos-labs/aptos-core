// u64 comparisons used as branch conditions, between two parameters and against a constant,
// each run with equal, smaller and larger operands. The golden pins how each one lowers.

// RUN: publish --print(stackless,micro-ops)
module 0x1::u64_cmp_jumps {
    fun le_rr(lhs: u64, rhs: u64): u64 { if (lhs <= rhs) 1 else 2 }
    fun ge_rr(lhs: u64, rhs: u64): u64 { if (lhs >= rhs) 1 else 2 }
    fun lt_rr(lhs: u64, rhs: u64): u64 { if (lhs < rhs) 1 else 2 }
    fun gt_rr(lhs: u64, rhs: u64): u64 { if (lhs > rhs) 1 else 2 }
    fun eq_rr(lhs: u64, rhs: u64): u64 { if (lhs == rhs) 1 else 2 }
    fun ne_rr(lhs: u64, rhs: u64): u64 { if (lhs != rhs) 1 else 2 }
    fun ge_imm(value: u64): u64 { if (value >= 10) 1 else 2 }
    fun lt_imm(value: u64): u64 { if (value < 10) 1 else 2 }
    fun eq_imm(value: u64): u64 { if (value == 10) 1 else 2 }
    fun ne_imm(value: u64): u64 { if (value != 10) 1 else 2 }
}

// RUN: execute 0x1::u64_cmp_jumps::le_rr --args 5, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::le_rr --args 4, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::le_rr --args 6, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::le_rr --args 0, 18446744073709551615
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::le_rr --args 18446744073709551615, 0
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ge_rr --args 5, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ge_rr --args 4, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ge_rr --args 6, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::lt_rr --args 5, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::lt_rr --args 4, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::lt_rr --args 6, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::gt_rr --args 5, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::gt_rr --args 4, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::gt_rr --args 6, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::eq_rr --args 5, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::eq_rr --args 4, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::eq_rr --args 6, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ne_rr --args 5, 5
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ne_rr --args 4, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ne_rr --args 6, 5
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ge_imm --args 9
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ge_imm --args 10
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ge_imm --args 11
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ge_imm --args 0
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ge_imm --args 18446744073709551615
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::lt_imm --args 9
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::lt_imm --args 10
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::lt_imm --args 11
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::eq_imm --args 9
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::eq_imm --args 10
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::eq_imm --args 11
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ne_imm --args 9
// CHECK: results: 1

// RUN: execute 0x1::u64_cmp_jumps::ne_imm --args 10
// CHECK: results: 2

// RUN: execute 0x1::u64_cmp_jumps::ne_imm --args 11
// CHECK: results: 1
