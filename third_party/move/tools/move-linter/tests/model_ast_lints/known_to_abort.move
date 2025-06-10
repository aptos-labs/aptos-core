module 0xc0ffee::m {
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
}
