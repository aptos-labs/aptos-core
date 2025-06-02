module 0xc0ffee::m {
    public fun test_divide_by_zero(x: u64): u64 {
        x / 0
    }

    public fun test_modulo_by_zero(x: u64): u64 {
        x % 0
    }

    public fun test_cast_u8_overflow(): u8 {
        256 as u8
    }

    public fun test_cast_u16_overflow(): u16 {
        70000 as u16
    }

    public fun test_cast_u8_from_negative(): u8 {
        300 as u8
    }

    public fun will_abort_if_casting(): u8 {
        let test: u16 = 1000;
        test as u8
    }

    public fun test_left_shift_u8_overflow(x: u8): u8 {
        x << 8
    }

    public fun test_right_shift_u8_overflow(x: u8): u8 {
        x >> 8
    }
}
