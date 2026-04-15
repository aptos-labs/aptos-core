module 0xc0ffee::range_full_domain_witness {
    fun full_range_with_bool(x: u8, b: bool): u64 {
        match ((x, b)) {
            (0..=255, true) => 1,
        }
    }
}
