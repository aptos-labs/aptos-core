// Integer faults on types other than u64: overflow against a constant operand, u8, i8, u16
// and u32 overflow and underflow, signed MIN / -1 and MIN % -1, signed remainder by zero,
// and variable shift amounts >= the bit width. Boundary results that succeed come first.

// RUN: publish --print(micro-ops)
module 0x1::unspecialized_errors {
    fun add_u8_imm(value: u8): u8 { value + 250 }
    fun mul_u128_imm(value: u128): u128 { value * 1000 }
    fun sub_i32_imm(value: i32): i32 { value - 10 }
    fun add_u8(lhs: u8, rhs: u8): u8 { lhs + rhs }
    fun sub_u8(lhs: u8, rhs: u8): u8 { lhs - rhs }
    fun mul_u8(lhs: u8, rhs: u8): u8 { lhs * rhs }
    fun add_i8(lhs: i8, rhs: i8): i8 { lhs + rhs }
    fun sub_i8(lhs: i8, rhs: i8): i8 { lhs - rhs }
    fun mul_i8(lhs: i8, rhs: i8): i8 { lhs * rhs }
    fun sub_u16(lhs: u16, rhs: u16): u16 { lhs - rhs }
    fun sub_u32(lhs: u32, rhs: u32): u32 { lhs - rhs }
    fun div_i8(lhs: i8, rhs: i8): i8 { lhs / rhs }
    fun div_i16(lhs: i16, rhs: i16): i16 { lhs / rhs }
    fun div_i32(lhs: i32, rhs: i32): i32 { lhs / rhs }
    fun div_i128(lhs: i128, rhs: i128): i128 { lhs / rhs }
    fun div_i256(lhs: i256, rhs: i256): i256 { lhs / rhs }
    fun mod_i8(lhs: i8, rhs: i8): i8 { lhs % rhs }
    fun mod_i64(lhs: i64, rhs: i64): i64 { lhs % rhs }
    fun mod_i256(lhs: i256, rhs: i256): i256 { lhs % rhs }
    fun shl_u8(value: u8, amount: u8): u8 { value << amount }
    fun shr_u16(value: u16, amount: u8): u16 { value >> amount }
    fun shl_u32(value: u32, amount: u8): u32 { value << amount }
    fun shr_u128(value: u128, amount: u8): u128 { value >> amount }
    fun shl_u256(value: u256, amount: u8): u256 { value << amount }
}

// RUN: execute 0x1::unspecialized_errors::add_u8_imm --args 5
// CHECK: results: 255

// RUN: execute 0x1::unspecialized_errors::mul_u128_imm --args 340282366920938463463374607431768211
// CHECK: results: 340282366920938463463374607431768211000

// RUN: execute 0x1::unspecialized_errors::sub_i32_imm --args -2147483638
// CHECK: results: -2147483648

// RUN: execute 0x1::unspecialized_errors::div_i8 --args -128, 1
// CHECK: results: -128

// RUN: execute 0x1::unspecialized_errors::mod_i8 --args -7, 2
// CHECK: results: -1

// RUN: execute 0x1::unspecialized_errors::shl_u8 --args 255, 7
// CHECK: results: 128

// RUN: execute 0x1::unspecialized_errors::shl_u256 --args 1, 255
// CHECK: results: 57896044618658097711785492504343953926634992332820282019728792003956564819968

// RUN: execute 0x1::unspecialized_errors::shr_u128 --args 340282366920938463463374607431768211455, 127
// CHECK: results: 1

// RUN: execute 0x1::unspecialized_errors::add_u8_imm --args 6
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Add: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mul_u128_imm --args 340282366920938463463374607431768212
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mul: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::sub_i32_imm --args -2147483639
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::add_u8 --args 200, 56
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Add: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::sub_u8 --args 0, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mul_u8 --args 16, 16
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mul: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::add_i8 --args 127, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Add: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::sub_i8 --args -128, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mul_i8 --args -128, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mul: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::sub_u16 --args 0, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::sub_u32 --args 1, 2
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::div_i8 --args -128, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::div_i16 --args -32768, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::div_i32 --args -2147483648, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::div_i128 --args -170141183460469231731687303715884105728, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::div_i256 --args -57896044618658097711785492504343953926634992332820282019728792003956564819968, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Div: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i8 --args -128, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i64 --args -9223372036854775808, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i256 --args -57896044618658097711785492504343953926634992332820282019728792003956564819968, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i8 --args 5, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i64 --args -5, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::mod_i256 --args 5, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: division by zero
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::shl_u8 --args 1, 8
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u8: shift amount 8 >= bit width 8
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::shr_u16 --args 1, 16
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u16: shift amount 16 >= bit width 16
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::shl_u32 --args 1, 32
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shl.u32: shift amount 32 >= bit width 32
// CHECK-ERROR-PARITY

// RUN: execute 0x1::unspecialized_errors::shr_u128 --args 1, 128
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Shr.u128: shift amount 128 >= bit width 128
// CHECK-ERROR-PARITY
