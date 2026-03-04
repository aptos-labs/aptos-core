module 0xc0ffee::int_non_exhaustive_witness {
    fun missing_wildcard(x: u8) {
        match (x) {
            0 => {},
            1 => {},
            2 => {},
        }
    }
}
