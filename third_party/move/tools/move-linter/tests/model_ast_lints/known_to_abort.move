module 0xc0ffee::m {
    // Constants for testing
    const OVERFLOW_VALUE: u16 = 256;
    const LARGE_U32: u64 = 4_294_967_296;
    const SHIFT_AMOUNT: u8 = 8;

    public fun test_divide_by_zero(x: u64): u64 {
        x / 0
    }

    public fun test_modulo_by_zero(x: u64): u64 {
        x % 0
    }

    public fun test_cast_u8_overflow(): u8 {
        256u16 as u8
    }

    public fun test_cast_u16_overflow(): u16 {
        65_536u32 as u16
    }

    public fun test_cast_u32_overflow(): u32 {
        4_294_967_296u64 as u32
    }

    public fun test_cast_u64_overflow(): u64 {
        18_446_744_073_709_551_616u128 as u64
    }

    public fun test_cast_u128_overflow(): u128 {
        340_282_366_920_938_463_463_374_607_431_768_211_456u256 as u128
    }

    public fun test_const_cast_overflow(): u8 {
        OVERFLOW_VALUE as u8
    }

    public fun test_const_large_cast(): u32 {
        LARGE_U32 as u32
    }

    public fun test_expr_cast_overflow(): u8 {
        (255u16 + 1u16) as u8
    }

    public fun test_expr_cast_overflow_2(): u16 {
        (65535u32 + 1u32) as u16
    }

    public fun test_expr_cast_overflow_3(): u32 {
        (4_294_967_295u64 + 1u64) as u32
    }

    public fun test_mul_expr_cast_overflow(): u8 {
        (128u8 * 2) as u8
    }

    public fun test_mul_expr_cast_overflow_2(): u16 {
        (32768u16 * 2) as u16
    }

    public fun test_mixed_expr_cast_overflow(): u8 {
        (128u8 + 128u8) as u8  // 256, overflows u8
    }

    public fun test_mixed_expr_cast_overflow_2(): u16 {
        (32768u16 * 2 + 1u16) as u16  // 65537, overflows u16
    }

    public fun test_nested_mul_expr_cast_overflow(): u8 {
        (64u8 * 2 * 2) as u8  // 256, overflows u8
    }

    public fun test_complex_expr_1(flag: bool): u8 {
        if (flag) {
            256u16 as u8  // This should be caught
        } else {
            255u16 as u8  // This should be fine
        }
    }

    public fun test_left_shift_u8_overflow(x: u8): u8 {
        x << 8
    }

    public fun test_left_shift_u16_overflow(x: u16): u16 {
        x << 16
    }

    public fun test_left_shift_u32_overflow(x: u32): u32 {
        x << 32
    }

    public fun test_left_shift_u64_overflow(x: u64): u64 {
        x << 64
    }

    public fun test_left_shift_u128_overflow(x: u128): u128 {
        x << 128
    }

    public fun test_shift_const_amount(x: u8): u8 {
        x << SHIFT_AMOUNT
    }

    public fun test_right_shift_u8_overflow(x: u8): u8 {
        x >> 8
    }

    public fun test_right_shift_u16_overflow(x: u16): u16 {
        x >> 16
    }

    public fun test_right_shift_u32_overflow(x: u32): u32 {
        x >> 32
    }

    public fun test_right_shift_u64_overflow(x: u64): u64 {
        x >> 64
    }

    public fun test_right_shift_u128_overflow(x: u128): u128 {
        x >> 128
    }

    // ===== LINT SKIP TESTS =====

    #[lint::skip(known_to_abort)]
    public fun test_divide_by_zero_skip(x: u64): u64 {
        x / 0
    }

    #[lint::skip(known_to_abort)]
    public fun test_modulo_by_zero_skip(x: u64): u64 {
        x % 0
    }

    #[lint::skip(known_to_abort)]
    public fun test_cast_u8_overflow_skip(): u8 {
        256u16 as u8
    }

    #[lint::skip(known_to_abort)]
    public fun test_cast_u16_overflow_skip(): u16 {
        70000u32 as u16
    }

    // ===== NEGATIVE TESTS (should NOT trigger lint) =====

    public fun test_complex_expr_1(x: u8): u8 {
        ((x + 1) * 2) as u8
    }

    public fun test_max_u8_cast(): u8 {
        255u16 as u8
    }

    public fun test_max_u16_cast(): u16 {
        65535u32 as u16
    }

    public fun test_max_u32_cast(): u32 {
        4294967295u64 as u32
    }

    public fun test_max_u64_cast(): u64 {
        18446744073709551615u128 as u64
    }

    public fun test_valid_mixed_expr(): u8 {
        (64u8 * 2 + 32) as u8  // 160, within u8 range
    }

    public fun test_valid_nested_mul(): u8 {
        (16u8 * 4 * 2) as u8  // 128, within u8 range
    }

    public fun test_valid_large_expr(): u16 {
        (32767u16 + 1 - 1) as u16  // 32767, within u16 range
    }

    public fun test_conditional_cast(x: u8, y: u8): u8 {
        if (x > y) {
            (x * 2) as u8
        } else {
            (y * 2) as u8
        }
    }

    public fun test_nested_conditional_cast(x: u8, y: u8, z: u8): u8 {
        if (x > y) {
            if (y > z) {
                (x + y) as u8
            } else {
                (x + z) as u8
            }
        } else {
            (y + z) as u8
        }
    }

    public fun test_valid_shift_u8(x: u8): u8 {
        x << 1
    }

    public fun test_valid_shift_u16(x: u16): u16 {
        x << 14
    }

    public fun test_valid_shift_u32(x: u32): u32 {
        x << 30
    }

    public fun test_valid_shift_u64(x: u64): u64 {
        x << 62
    }

    public fun test_valid_shift_u128(x: u128): u128 {
        x << 126
    }

    public fun test_valid_shift_u256(x: u256): u256 {
        x << 254
    }
}
