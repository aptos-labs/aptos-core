// Arithmetic faults: u64 subtraction below 0 (by a parameter and by the constant 1), signed
// MIN % -1 for i16, i32 and i128, and negation of MIN for i8 and i32. Most sit next to a call
// that succeeds.

// RUN: publish
module 0x66::u64_faults {
    fun sub(lhs: u64, rhs: u64): u64 { lhs - rhs }
    fun dec(value: u64): u64 { value - 1 }
    fun i8_rem(lhs: i8, rhs: i8): i8 { lhs % rhs }
    fun i16_rem(lhs: i16, rhs: i16): i16 { lhs % rhs }
    fun i32_rem(lhs: i32, rhs: i32): i32 { lhs % rhs }
    fun i128_rem(lhs: i128, rhs: i128): i128 { lhs % rhs }
    fun i8_neg(value: i8): i8 { -value }
    fun i32_neg(value: i32): i32 { -value }
}

// RUN: execute 0x66::u64_faults::sub --args 0, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub.u64: underflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::dec --args 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Sub.u64: underflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::dec --args 1
// CHECK: results: 0

// RUN: execute 0x66::u64_faults::i8_rem --args -127, -1
// CHECK: results: 0

// RUN: execute 0x66::u64_faults::i16_rem --args -32768, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::i32_rem --args -2147483648, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::i128_rem --args -170141183460469231731687303715884105728, -1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Mod: MIN divided by -1 overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::i8_neg --args -128
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Negate.i8: Negate of MIN overflows
// CHECK-ERROR-PARITY

// RUN: execute 0x66::u64_faults::i8_neg --args -127
// CHECK: results: 127

// RUN: execute 0x66::u64_faults::i32_neg --args -2147483648
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: Negate.i32: Negate of MIN overflows
// CHECK-ERROR-PARITY
