// Shifts by a variable amount at and past each integer width: width - 1 succeeds and the
// width itself fails with ARITHMETIC_ERROR.

// RUN: publish
module 0x66::shift_bounds {
    fun shl8(value: u8, amount: u8): u8 { value << amount }
    fun shr8(value: u8, amount: u8): u8 { value >> amount }
    fun shl16(value: u16, amount: u8): u16 { value << amount }
    fun shr16(value: u16, amount: u8): u16 { value >> amount }
    fun shl32(value: u32, amount: u8): u32 { value << amount }
    fun shr32(value: u32, amount: u8): u32 { value >> amount }
    fun shl64(value: u64, amount: u8): u64 { value << amount }
    fun shl128(value: u128, amount: u8): u128 { value << amount }
    fun shr128(value: u128, amount: u8): u128 { value >> amount }
    fun shr256(value: u256, amount: u8): u256 { value >> amount }

    // Constant amounts at the width, on a variable operand.
    fun shl8_const(value: u8): u8 { value << 8 }
    fun shr16_const(value: u16): u16 { value >> 16 }
    fun shl32_const(value: u32): u32 { value << 32 }
    fun shr128_const(value: u128): u128 { value >> 128 }
}

// RUN: execute 0x66::shift_bounds::shl8 --args 1, 7
// CHECK: results: 128

// RUN: execute 0x66::shift_bounds::shr8 --args 255, 7
// CHECK: results: 1

// RUN: execute 0x66::shift_bounds::shr8 --args 255, 8
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u8: shift amount 8 >= bit width 8
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shl16 --args 1, 15
// CHECK: results: 32768

// RUN: execute 0x66::shift_bounds::shl16 --args 1, 16
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u16: shift amount 16 >= bit width 16
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shr16 --args 65535, 15
// CHECK: results: 1

// RUN: execute 0x66::shift_bounds::shr16 --args 65535, 16
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u16: shift amount 16 >= bit width 16
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shl32 --args 1, 31
// CHECK: results: 2147483648

// RUN: execute 0x66::shift_bounds::shr32 --args 4294967295, 31
// CHECK: results: 1

// RUN: execute 0x66::shift_bounds::shr32 --args 4294967295, 32
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u32: shift amount 32 >= bit width 32
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shl64 --args 1, 63
// CHECK: results: 9223372036854775808

// RUN: execute 0x66::shift_bounds::shl128 --args 1, 127
// CHECK: results: 170141183460469231731687303715884105728

// RUN: execute 0x66::shift_bounds::shl128 --args 1, 128
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u128: shift amount 128 >= bit width 128
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shr128 --args 340282366920938463463374607431768211455, 128
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u128: shift amount 128 >= bit width 128
// CHECK-ERROR-PARITY

// 136 is 0 mod 8: the whole amount is checked, not just its low bits.

// RUN: execute 0x66::shift_bounds::shr8 --args 1, 136
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u8: shift amount 136 >= bit width 8
// CHECK-ERROR-PARITY

// u256 accepts every u8 amount.

// RUN: execute 0x66::shift_bounds::shr256 --args 115792089237316195423570985008687907853269984665640564039457584007913129639935, 255
// CHECK: results: 1

// Constant amounts at the width.

// RUN: execute 0x66::shift_bounds::shl8_const --args 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u8: shift amount 8 >= bit width 8
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shr16_const --args 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u16: shift amount 16 >= bit width 16
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shl32_const --args 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u32: shift amount 32 >= bit width 32
// CHECK-ERROR-PARITY

// RUN: execute 0x66::shift_bounds::shr128_const --args 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u128: shift amount 128 >= bit width 128
// CHECK-ERROR-PARITY
