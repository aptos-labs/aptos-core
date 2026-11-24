// Add, sub and mul on two parameters at u32, i16, i256 and u256. Each fault sits next to a
// call that reaches the largest or smallest representable value. Both VMs name the
// operation in their error message.

// RUN: publish
module 0x66::checked_overflow_widths {
    fun add_u32(lhs: u32, rhs: u32): u32 { lhs + rhs }
    fun mul_u32(lhs: u32, rhs: u32): u32 { lhs * rhs }
    fun add_i16(lhs: i16, rhs: i16): i16 { lhs + rhs }
    fun sub_i16(lhs: i16, rhs: i16): i16 { lhs - rhs }
    fun mul_i16(lhs: i16, rhs: i16): i16 { lhs * rhs }
    fun add_i256(lhs: i256, rhs: i256): i256 { lhs + rhs }
    fun sub_i256(lhs: i256, rhs: i256): i256 { lhs - rhs }
    fun sub_u256(lhs: u256, rhs: u256): u256 { lhs - rhs }
    fun mul_u256(lhs: u256, rhs: u256): u256 { lhs * rhs }
}

// RUN: execute 0x66::checked_overflow_widths::add_u32 --args 4294967000, 295
// CHECK: results: 4294967295

// RUN: execute 0x66::checked_overflow_widths::add_u32 --args 4294967000, 296
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Addition overflow
// CHECK-V2: error: Add: under/overflow
// CHECK-ERROR-PARITY

// 65535 * 65537 = 2^32 - 1.
// RUN: execute 0x66::checked_overflow_widths::mul_u32 --args 65535, 65537
// CHECK: results: 4294967295

// RUN: execute 0x66::checked_overflow_widths::mul_u32 --args 65536, 65536
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Multiplication overflow
// CHECK-V2: error: Mul: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::checked_overflow_widths::add_i16 --args 32000, 767
// CHECK: results: 32767

// RUN: execute 0x66::checked_overflow_widths::add_i16 --args 32000, 768
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Addition overflow
// CHECK-V2: error: Add: under/overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::checked_overflow_widths::sub_i16 --args -32000, 768
// CHECK: results: -32768

// RUN: execute 0x66::checked_overflow_widths::sub_i16 --args -32000, 769
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Subtraction overflow
// CHECK-V2: error: Sub: under/overflow
// CHECK-ERROR-PARITY

// -128 * 256 is the minimum; 128 * 256 is one above the maximum.
// RUN: execute 0x66::checked_overflow_widths::mul_i16 --args -128, 256
// CHECK: results: -32768

// RUN: execute 0x66::checked_overflow_widths::mul_i16 --args 128, 256
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Multiplication overflow
// CHECK-V2: error: Mul: under/overflow
// CHECK-ERROR-PARITY

// 2^254 + (2^254 - 1) is the maximum; 2^254 + 2^254 is one above it.
// RUN: execute 0x66::checked_overflow_widths::add_i256 --args 28948022309329048855892746252171976963317496166410141009864396001978282409984, 28948022309329048855892746252171976963317496166410141009864396001978282409983
// CHECK: results: 57896044618658097711785492504343953926634992332820282019728792003956564819967

// RUN: execute 0x66::checked_overflow_widths::add_i256 --args 28948022309329048855892746252171976963317496166410141009864396001978282409984, 28948022309329048855892746252171976963317496166410141009864396001978282409984
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Addition overflow
// CHECK-V2: error: Add: under/overflow
// CHECK-ERROR-PARITY

// -2^254 - 2^254 is the minimum; -2^254 - (2^254 + 1) is one below it.
// RUN: execute 0x66::checked_overflow_widths::sub_i256 --args -28948022309329048855892746252171976963317496166410141009864396001978282409984, 28948022309329048855892746252171976963317496166410141009864396001978282409984
// CHECK: results: -57896044618658097711785492504343953926634992332820282019728792003956564819968

// RUN: execute 0x66::checked_overflow_widths::sub_i256 --args -28948022309329048855892746252171976963317496166410141009864396001978282409984, 28948022309329048855892746252171976963317496166410141009864396001978282409985
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Subtraction overflow
// CHECK-V2: error: Sub: under/overflow
// CHECK-ERROR-PARITY

// Around 2^192 the borrow crosses 64-bit words: 2^192 - (2^192 - 1) = 1,
// 2^192 - 2^192 = 0, and 2^192 - (2^192 + 1) underflows.
// RUN: execute 0x66::checked_overflow_widths::sub_u256 --args 6277101735386680763835789423207666416102355444464034512896, 6277101735386680763835789423207666416102355444464034512895
// CHECK: results: 1

// RUN: execute 0x66::checked_overflow_widths::sub_u256 --args 6277101735386680763835789423207666416102355444464034512896, 6277101735386680763835789423207666416102355444464034512896
// CHECK: results: 0

// RUN: execute 0x66::checked_overflow_widths::sub_u256 --args 6277101735386680763835789423207666416102355444464034512896, 6277101735386680763835789423207666416102355444464034512897
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Subtraction overflow
// CHECK-V2: error: Sub: under/overflow
// CHECK-ERROR-PARITY

// (2^128 - 1) * (2^128 + 1) = 2^256 - 1 is the maximum; 2^128 * 2^128 = 2^256 is one above it.
// RUN: execute 0x66::checked_overflow_widths::mul_u256 --args 340282366920938463463374607431768211455, 340282366920938463463374607431768211457
// CHECK: results: 115792089237316195423570985008687907853269984665640564039457584007913129639935

// RUN: execute 0x66::checked_overflow_widths::mul_u256 --args 340282366920938463463374607431768211456, 340282366920938463463374607431768211456
// CHECK-V1-SUBSTR: status ARITHMETIC_ERROR
// CHECK-V1-SUBSTR: and message Multiplication overflow
// CHECK-V2: error: Mul: under/overflow
// CHECK-ERROR-PARITY
