// Zero u64 divisors and shift amounts >= 64 lower to checked integer ops
// and produce the same runtime errors as V1.

// RUN: publish --print(micro-ops)
module 0x1::fused_imm_faulting_constants {
    fun div_by_zero(x: u64): u64 { x / 0 }
    fun mod_by_zero(x: u64): u64 { x % 0 }
    fun shl_by_64(x: u64): u64 { x << 64 }
    fun shr_by_64(x: u64): u64 { x >> 64 }
}

// RUN: execute 0x1::fused_imm_faulting_constants::div_by_zero --args 7
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::fused_imm_faulting_constants::mod_by_zero --args 7
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::fused_imm_faulting_constants::shl_by_64 --args 7
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u64: shift amount 64 >= bit width 64
// CHECK-ERROR-PARITY

// RUN: execute 0x1::fused_imm_faulting_constants::shr_by_64 --args 7
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u64: shift amount 64 >= bit width 64
// CHECK-ERROR-PARITY
