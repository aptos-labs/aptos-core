module 0xc0ffee::primitive_match_not_supported {
    public fun test(x: u8): u8 {
        match (x) {
            1 => 1,
            _ => 0,
        }
    }
}
