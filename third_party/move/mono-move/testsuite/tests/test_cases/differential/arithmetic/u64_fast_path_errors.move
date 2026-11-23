// u64 arithmetic and shifts at the edge of their range: boundary results first, then the
// faults (overflow, underflow, division by zero, shift amount >= 64). Operands are
// parameters, except in `sub_imm`, which subtracts a constant.

// RUN: publish --print(micro-ops)
module 0x1::u64_fast_path_errors {
    fun add(lhs: u64, rhs: u64): u64 { lhs + rhs }
    fun mul(lhs: u64, rhs: u64): u64 { lhs * rhs }
    fun sub(lhs: u64, rhs: u64): u64 { lhs - rhs }
    fun sub_imm(value: u64): u64 { value - 5 }
    fun div(lhs: u64, rhs: u64): u64 { lhs / rhs }
    fun mod(lhs: u64, rhs: u64): u64 { lhs % rhs }
    fun shl(value: u64, amount: u8): u64 { value << amount }
    fun shr(value: u64, amount: u8): u64 { value >> amount }
}

// RUN: execute 0x1::u64_fast_path_errors::add --args 18446744073709551614, 1
// CHECK: results: 18446744073709551615

// RUN: execute 0x1::u64_fast_path_errors::mul --args 4294967296, 4294967295
// CHECK: results: 18446744069414584320

// RUN: execute 0x1::u64_fast_path_errors::sub --args 5, 5
// CHECK: results: 0

// RUN: execute 0x1::u64_fast_path_errors::sub_imm --args 5
// CHECK: results: 0

// RUN: execute 0x1::u64_fast_path_errors::mod --args 18446744073709551615, 1
// CHECK: results: 0

// RUN: execute 0x1::u64_fast_path_errors::shl --args 18446744073709551615, 63
// CHECK: results: 9223372036854775808

// RUN: execute 0x1::u64_fast_path_errors::shl --args 1, 0
// CHECK: results: 1

// RUN: execute 0x1::u64_fast_path_errors::shr --args 18446744073709551615, 63
// CHECK: results: 1

// RUN: execute 0x1::u64_fast_path_errors::shr --args 18446744073709551615, 0
// CHECK: results: 18446744073709551615

// RUN: execute 0x1::u64_fast_path_errors::add --args 18446744073709551615, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Add.u64: overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::mul --args 4294967296, 4294967296
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mul.u64: overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::sub --args 4, 5
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub.u64: underflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::sub_imm --args 4
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub.u64: underflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::div --args 1, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::mod --args 7, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::shl --args 1, 64
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u64: shift amount 64 >= bit width 64
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::shl --args 1, 255
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u64: shift amount 255 >= bit width 64
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::shr --args 18446744073709551615, 64
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u64: shift amount 64 >= bit width 64
// CHECK-ERROR-PARITY

// RUN: execute 0x1::u64_fast_path_errors::shr --args 18446744073709551615, 200
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u64: shift amount 200 >= bit width 64
// CHECK-ERROR-PARITY
