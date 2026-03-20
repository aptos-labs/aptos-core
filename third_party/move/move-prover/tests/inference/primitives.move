// Test spec inference for primitive operations
module 0x42::primitives {

    // ==================== Arithmetic Operations ====================

    // Addition - should infer: ensures result == x + y; aborts_if overflow
    fun test_add(x: u64, y: u64): u64 {
        x + y
    }

    // Subtraction - should infer: ensures result == x - y; aborts_if underflow
    fun test_sub(x: u64, y: u64): u64 {
        x - y
    }

    // Multiplication - should infer: ensures result == x * y; aborts_if overflow
    fun test_mul(x: u64, y: u64): u64 {
        x * y
    }

    // Division - should infer: ensures result == x / y; aborts_if y == 0
    fun test_div(x: u64, y: u64): u64 {
        x / y
    }

    // Modulo - should infer: ensures result == x % y; aborts_if y == 0
    fun test_mod(x: u64, y: u64): u64 {
        x % y
    }

    // Chained arithmetic - should infer: ensures result == (x + y) * 2
    fun test_chain_arith(x: u64, y: u64): u64 {
        let sum = x + y;
        sum * 2
    }

    // ==================== Comparison Operations ====================

    // Equality - should infer: ensures result == (x == y)
    fun test_eq(x: u64, y: u64): bool {
        x == y
    }

    // Not equal - should infer: ensures result == (x != y)
    fun test_neq(x: u64, y: u64): bool {
        x != y
    }

    // Less than - should infer: ensures result == (x < y)
    fun test_lt(x: u64, y: u64): bool {
        x < y
    }

    // Less than or equal - should infer: ensures result == (x <= y)
    fun test_le(x: u64, y: u64): bool {
        x <= y
    }

    // Greater than - should infer: ensures result == (x > y)
    fun test_gt(x: u64, y: u64): bool {
        x > y
    }

    // Greater than or equal - should infer: ensures result == (x >= y)
    fun test_ge(x: u64, y: u64): bool {
        x >= y
    }

    // ==================== Logical Operations ====================

    // Logical AND - should infer: ensures result == (a && b)
    fun test_and(a: bool, b: bool): bool {
        a && b
    }

    // Logical OR - should infer: ensures result == (a || b)
    fun test_or(a: bool, b: bool): bool {
        a || b
    }

    // Logical NOT - should infer: ensures result == !a
    fun test_not(a: bool): bool {
        !a
    }

    // ==================== Bitwise Operations ====================

    // Bitwise OR - should infer: ensures result == (x | y)
    fun test_bit_or(x: u64, y: u64): u64 {
        x | y
    }

    // Bitwise AND - should infer: ensures result == (x & y)
    fun test_bit_and(x: u64, y: u64): u64 {
        x & y
    }

    // Bitwise XOR - should infer: ensures result == (x ^ y)
    fun test_xor(x: u64, y: u64): u64 {
        x ^ y
    }

    // Left shift - should infer: ensures result == (x << n); aborts_if n >= 64
    fun test_shl(x: u64, n: u8): u64 {
        x << n
    }

    // Right shift - should infer: ensures result == (x >> n); aborts_if n >= 64
    fun test_shr(x: u64, n: u8): u64 {
        x >> n
    }

    // ==================== Cast Operations ====================

    // Cast u64 to u8 - should infer: ensures result == (x as u8); aborts_if out of range
    fun test_cast_u8(x: u64): u8 {
        (x as u8)
    }

    // Cast u64 to u16 - should infer: ensures result == (x as u16); aborts_if out of range
    fun test_cast_u16(x: u64): u16 {
        (x as u16)
    }

    // Cast u64 to u32 - should infer: ensures result == (x as u32); aborts_if out of range
    fun test_cast_u32(x: u64): u32 {
        (x as u32)
    }

    // Cast u8 to u64 - should infer: ensures result == (x as u64); never aborts
    fun test_cast_u64(x: u8): u64 {
        (x as u64)
    }

    // Cast u64 to u128 - should infer: ensures result == (x as u128); never aborts
    fun test_cast_u128(x: u64): u128 {
        (x as u128)
    }

    // Cast u64 to u256 - should infer: ensures result == (x as u256); never aborts
    fun test_cast_u256(x: u64): u256 {
        (x as u256)
    }
}
