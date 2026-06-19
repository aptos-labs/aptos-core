// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    // Bare constant loads -> StoreImm2 / StoreImm4.
    fun c_u16(): u16 { 40000u16 }
    fun c_u32(): u32 { 3000000000u32 }
    fun c_i16(): i16 { 12345i16 }
    fun c_i32(): i32 { 2000000000i32 }

    // Arithmetic (reg-reg).
    fun add_u16(a: u16, b: u16): u16 { a + b }
    fun mul_u16(a: u16, b: u16): u16 { a * b }
    fun div_u32(a: u32, b: u32): u32 { a / b }
    fun add_i16(a: i16, b: i16): i16 { a + b }
    fun sub_i32(a: i32, b: i32): i32 { a - b }
    fun mul_i32(a: i32, b: i32): i32 { a * b }

    // Arithmetic against an immediate (destacker fuses `ld + binop`).
    fun add_u16_imm(a: u16): u16 { a + 1000 }
    fun mul_u32_imm(a: u32): u32 { a * 7 }

    // Comparison -> 1-byte bool.
    fun lt_u16(a: u16, b: u16): bool { a < b }
    fun lt_i16(a: i16, b: i16): bool { a < b }
    fun eq_u32(a: u32, b: u32): bool { a == b }

    // Bitwise (unsigned only).
    fun and_u16(a: u16, b: u16): u16 { a & b }
    fun xor_u32(a: u32, b: u32): u32 { a ^ b }

    // Shift: immediate amount and slot (u8) amount.
    fun shl_u16_imm(a: u16): u16 { a << 3 }
    fun shr_u32_var(a: u32, s: u8): u32 { a >> s }

    // Signed negate.
    fun neg_i16(a: i16): i16 { -a }
    fun neg_i32(a: i32): i32 { -a }
}

// --- Constant loads ---
// RUN: execute 0x1::test::c_u16
// CHECK: results: 40000
// RUN: execute 0x1::test::c_u32
// CHECK: results: 3000000000
// RUN: execute 0x1::test::c_i16
// CHECK: results: 12345
// RUN: execute 0x1::test::c_i32
// CHECK: results: 2000000000

// --- Arithmetic ---
// RUN: execute 0x1::test::add_u16 --args 100, 250
// CHECK: results: 350
// RUN: execute 0x1::test::mul_u16 --args 200, 3
// CHECK: results: 600
// RUN: execute 0x1::test::div_u32 --args 100, 7
// CHECK: results: 14
// RUN: execute 0x1::test::add_i16 --args -50, 20
// CHECK: results: -30
// RUN: execute 0x1::test::sub_i32 --args 5, 12
// CHECK: results: -7
// RUN: execute 0x1::test::mul_i32 --args -6, 7
// CHECK: results: -42

// Boundary values that stay in range.
// u16 MAX.
// RUN: execute 0x1::test::add_u16 --args 65534, 1
// CHECK: results: 65535
// i32 MIN - (-1).
// RUN: execute 0x1::test::sub_i32 --args -2147483648, -1
// CHECK: results: -2147483647

// --- Immediate forms ---
// RUN: execute 0x1::test::add_u16_imm --args 5
// CHECK: results: 1005
// RUN: execute 0x1::test::mul_u32_imm --args 6
// CHECK: results: 42

// --- Comparison ---
// RUN: execute 0x1::test::lt_u16 --args 1, 2
// CHECK: results: true
// RUN: execute 0x1::test::lt_u16 --args 2, 1
// CHECK: results: false
// Signed ordering: -1 < 1 is true
// RUN: execute 0x1::test::lt_i16 --args -1, 1
// CHECK: results: true
// RUN: execute 0x1::test::lt_i16 --args 1, -1
// CHECK: results: false
// RUN: execute 0x1::test::eq_u32 --args 7, 7
// CHECK: results: true
// RUN: execute 0x1::test::eq_u32 --args 7, 8
// CHECK: results: false

// --- Bitwise ---
// RUN: execute 0x1::test::and_u16 --args 12, 10
// CHECK: results: 8
// RUN: execute 0x1::test::xor_u32 --args 255, 240
// CHECK: results: 15

// --- Shift ---
// RUN: execute 0x1::test::shl_u16_imm --args 5
// CHECK: results: 40
// RUN: execute 0x1::test::shr_u32_var --args 48, 2
// CHECK: results: 12

// --- Negate ---
// RUN: execute 0x1::test::neg_i16 --args -42
// CHECK: results: 42
// RUN: execute 0x1::test::neg_i16 --args 100
// CHECK: results: -100
// RUN: execute 0x1::test::neg_i32 --args 2000000000
// CHECK: results: -2000000000

// --- Abort paths (V1 and V2 phrase aborts differently) ---
// u16 overflow.
// RUN: execute 0x1::test::add_u16 --args 65535, 1
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:
// u32 division by zero.
// RUN: execute 0x1::test::div_u32 --args 100, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:
// i32 multiply overflow.
// RUN: execute 0x1::test::mul_i32 --args 100000, 100000
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:
// Negate of i16::MIN overflows.
// RUN: execute 0x1::test::neg_i16 --args -32768
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-V2-SUBSTR: error:
