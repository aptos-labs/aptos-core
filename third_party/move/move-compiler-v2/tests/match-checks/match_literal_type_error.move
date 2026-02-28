module 0xc0ffee::match_literal_type_error {
    fun match_bool_with_number(x: bool): u64 {
        match (x) {
            1 => 1,
            _ => 0,
        }
    }

    fun match_u8_with_u64(x: u8): u64 {
        match (x) {
            1u64 => 1,
            _ => 0,
        }
    }
}
