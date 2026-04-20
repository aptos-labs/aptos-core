module 0xc0ffee::range_open_ended_empty_error {
    // ..0 on u8: effective range is [0, 0) which is empty
    fun open_start_at_u8_min(x: u8): u64 {
        match (x) {
            ..0 => 1,
            _ => 0,
        }
    }

    // ..-128 on i8: effective range is [-128, -128) which is empty
    fun open_start_at_i8_min(x: i8): u64 {
        match (x) {
            ..-128i8 => 1,
            _ => 0,
        }
    }

    // ..0 on u64: effective range is [0, 0) which is empty
    fun open_start_at_u64_min(x: u64): u64 {
        match (x) {
            ..0 => 1,
            _ => 0,
        }
    }

    // ..0 on u16: effective range is [0, 0) which is empty
    fun open_start_at_u16_min(x: u16): u64 {
        match (x) {
            ..0u16 => 1,
            _ => 0,
        }
    }
}
