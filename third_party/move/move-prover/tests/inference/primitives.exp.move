// Test spec inference for primitive operations
module 0x42::primitives {

    // ==================== Arithmetic Operations ====================

    // Addition - should infer: ensures result == x + y; aborts_if overflow
    fun test_add(x: u64, y: u64): u64 {
        x + y
    }
    spec test_add(x: u64, y: u64): u64 {
        ensures [inferred] result == x + y;
        aborts_if [inferred] x + y > MAX_U64;
    }


    // Subtraction - should infer: ensures result == x - y; aborts_if underflow
    fun test_sub(x: u64, y: u64): u64 {
        x - y
    }
    spec test_sub(x: u64, y: u64): u64 {
        ensures [inferred] result == x - y;
        aborts_if [inferred] x - y < 0;
    }


    // Multiplication - should infer: ensures result == x * y; aborts_if overflow
    fun test_mul(x: u64, y: u64): u64 {
        x * y
    }
    spec test_mul(x: u64, y: u64): u64 {
        ensures [inferred] result == x * y;
        aborts_if [inferred] x * y > MAX_U64;
    }


    // Division - should infer: ensures result == x / y; aborts_if y == 0
    fun test_div(x: u64, y: u64): u64 {
        x / y
    }
    spec test_div(x: u64, y: u64): u64 {
        ensures [inferred] result == x / y;
        aborts_if [inferred] y == 0;
    }


    // Modulo - should infer: ensures result == x % y; aborts_if y == 0
    fun test_mod(x: u64, y: u64): u64 {
        x % y
    }
    spec test_mod(x: u64, y: u64): u64 {
        ensures [inferred] result == x % y;
        aborts_if [inferred] y == 0;
    }


    // Chained arithmetic - should infer: ensures result == (x + y) * 2
    fun test_chain_arith(x: u64, y: u64): u64 {
        let sum = x + y;
        sum * 2
    }
    spec test_chain_arith(x: u64, y: u64): u64 {
        ensures [inferred] result == (x + y) * 2;
        aborts_if [inferred] (x + y) * 2 > MAX_U64;
        aborts_if [inferred] x + y > MAX_U64;
    }


    // ==================== Comparison Operations ====================

    // Equality - should infer: ensures result == (x == y)
    fun test_eq(x: u64, y: u64): bool {
        x == y
    }
    spec test_eq(x: u64, y: u64): bool {
        ensures [inferred] result == (x == y);
    }


    // Not equal - should infer: ensures result == (x != y)
    fun test_neq(x: u64, y: u64): bool {
        x != y
    }
    spec test_neq(x: u64, y: u64): bool {
        ensures [inferred] result == (x != y);
    }


    // Less than - should infer: ensures result == (x < y)
    fun test_lt(x: u64, y: u64): bool {
        x < y
    }
    spec test_lt(x: u64, y: u64): bool {
        ensures [inferred] result == (x < y);
    }


    // Less than or equal - should infer: ensures result == (x <= y)
    fun test_le(x: u64, y: u64): bool {
        x <= y
    }
    spec test_le(x: u64, y: u64): bool {
        ensures [inferred] result == (x <= y);
    }


    // Greater than - should infer: ensures result == (x > y)
    fun test_gt(x: u64, y: u64): bool {
        x > y
    }
    spec test_gt(x: u64, y: u64): bool {
        ensures [inferred] result == (x > y);
    }


    // Greater than or equal - should infer: ensures result == (x >= y)
    fun test_ge(x: u64, y: u64): bool {
        x >= y
    }
    spec test_ge(x: u64, y: u64): bool {
        ensures [inferred] result == (x >= y);
    }


    // ==================== Logical Operations ====================

    // Logical AND - should infer: ensures result == (a && b)
    fun test_and(a: bool, b: bool): bool {
        a && b
    }
    spec test_and(a: bool, b: bool): bool {
        ensures [inferred] a ==> result == b;
        ensures [inferred] !a ==> result == false;
    }


    // Logical OR - should infer: ensures result == (a || b)
    fun test_or(a: bool, b: bool): bool {
        a || b
    }
    spec test_or(a: bool, b: bool): bool {
        ensures [inferred] a ==> result == true;
        ensures [inferred] !a ==> result == b;
    }


    // Logical NOT - should infer: ensures result == !a
    fun test_not(a: bool): bool {
        !a
    }
    spec test_not(a: bool): bool {
        ensures [inferred] result == !a;
    }


    // ==================== Bitwise Operations ====================

    // Bitwise OR - should infer: ensures result == (x | y)
    fun test_bit_or(x: u64, y: u64): u64 {
        x | y
    }
    spec test_bit_or(x: u64, y: u64): u64 {
        ensures [inferred] result == x | y;
    }


    // Bitwise AND - should infer: ensures result == (x & y)
    fun test_bit_and(x: u64, y: u64): u64 {
        x & y
    }
    spec test_bit_and(x: u64, y: u64): u64 {
        ensures [inferred] result == x & y;
    }


    // Bitwise XOR - should infer: ensures result == (x ^ y)
    fun test_xor(x: u64, y: u64): u64 {
        x ^ y
    }
    spec test_xor(x: u64, y: u64): u64 {
        ensures [inferred] result == x ^ y;
    }


    // Left shift - should infer: ensures result == (x << n); aborts_if n >= 64
    fun test_shl(x: u64, n: u8): u64 {
        x << n
    }
    spec test_shl(x: u64, n: u8): u64 {
        ensures [inferred] result == x << n;
        aborts_if [inferred] n >= 64;
    }


    // Right shift - should infer: ensures result == (x >> n); aborts_if n >= 64
    fun test_shr(x: u64, n: u8): u64 {
        x >> n
    }
    spec test_shr(x: u64, n: u8): u64 {
        ensures [inferred] result == x >> n;
        aborts_if [inferred] n >= 64;
    }


    // ==================== Cast Operations ====================

    // Cast u64 to u8 - should infer: ensures result == (x as u8); aborts_if out of range
    fun test_cast_u8(x: u64): u8 {
        (x as u8)
    }
    spec test_cast_u8(x: u64): u8 {
        ensures [inferred] result == (x as u8);
        aborts_if [inferred] x > MAX_U8;
    }


    // Cast u64 to u16 - should infer: ensures result == (x as u16); aborts_if out of range
    fun test_cast_u16(x: u64): u16 {
        (x as u16)
    }
    spec test_cast_u16(x: u64): u16 {
        ensures [inferred] result == (x as u16);
        aborts_if [inferred] x > MAX_U16;
    }


    // Cast u64 to u32 - should infer: ensures result == (x as u32); aborts_if out of range
    fun test_cast_u32(x: u64): u32 {
        (x as u32)
    }
    spec test_cast_u32(x: u64): u32 {
        ensures [inferred] result == (x as u32);
        aborts_if [inferred] x > MAX_U32;
    }


    // Cast u8 to u64 - should infer: ensures result == (x as u64); never aborts
    fun test_cast_u64(x: u8): u64 {
        (x as u64)
    }
    spec test_cast_u64(x: u8): u64 {
        ensures [inferred] result == (x as u64);
    }


    // Cast u64 to u128 - should infer: ensures result == (x as u128); never aborts
    fun test_cast_u128(x: u64): u128 {
        (x as u128)
    }
    spec test_cast_u128(x: u64): u128 {
        ensures [inferred] result == (x as u128);
    }


    // Cast u64 to u256 - should infer: ensures result == (x as u256); never aborts
    fun test_cast_u256(x: u64): u256 {
        (x as u256)
    }
    spec test_cast_u256(x: u64): u256 {
        ensures [inferred] result == (x as u256);
    }

}
/*
Verification: Succeeded.
*/
