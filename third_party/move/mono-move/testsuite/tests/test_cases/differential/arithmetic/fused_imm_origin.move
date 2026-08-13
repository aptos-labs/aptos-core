// RUN: publish --print(stackless,micro-ops)
module 0x1::fused_imm_origin {
    fun bump(x: u64): u64 { x + 1 }
    fun scale(x: u64): u64 { x * 3 }
}

// RUN: execute 0x1::fused_imm_origin::bump --args 5
// CHECK: results: 6

// Overflow inside the fused add-immediate.
// RUN: execute 0x1::fused_imm_origin::bump --args 18446744073709551615
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: overflow

// RUN: execute 0x1::fused_imm_origin::scale --args 7
// CHECK: results: 21

// Overflow inside the fused multiply-immediate.
// RUN: execute 0x1::fused_imm_origin::scale --args 9223372036854775807
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: overflow
