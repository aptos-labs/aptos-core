// Differential coverage for bitwise + shift on the unspecialized
// integer path. u64 ops still use the specialized fast path; the widths
// here (u128, u256) flow through `IntBitAnd`/etc. and `IntShl`/`IntShr`.
//
// Only the immediate-shift form is exercised here: Move's shift rhs is
// `u8`, and the current home-slot layout rejects align-1 types (see
// `MIN_SLOT_ALIGN` in `lower::context`). Once small-type slots are
// supported the slot-based shift form (`IntShl::Slot`) can be added.

// RUN: publish
module 0x1::test {
    fun u128_and(a: u128, b: u128): u128 { a & b }
    fun u128_or(a: u128, b: u128): u128 { a | b }
    fun u128_xor(a: u128, b: u128): u128 { a ^ b }
    fun u256_xor(a: u256, b: u256): u256 { a ^ b }

    fun u128_shl(a: u128): u128 { a << 3 }
    fun u128_shr(a: u128): u128 { a >> 2 }
    fun u256_shl(a: u256): u256 { a << 5 }
    fun u256_shr(a: u256): u256 { a >> 4 }
}

// RUN: execute 0x1::test::u128_and --args 12, 10
// CHECK: results: 8

// RUN: execute 0x1::test::u128_or --args 12, 10
// CHECK: results: 14

// RUN: execute 0x1::test::u128_xor --args 12, 10
// CHECK: results: 6

// RUN: execute 0x1::test::u256_xor --args 255, 240
// CHECK: results: 15

// RUN: execute 0x1::test::u128_shl --args 5
// CHECK: results: 40

// RUN: execute 0x1::test::u128_shr --args 48
// CHECK: results: 12

// RUN: execute 0x1::test::u256_shl --args 7
// CHECK: results: 224

// RUN: execute 0x1::test::u256_shr --args 256
// CHECK: results: 16
