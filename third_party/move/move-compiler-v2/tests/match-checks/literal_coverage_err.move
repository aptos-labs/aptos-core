module 0xc0ffee::literal_coverage_err {
    fun non_exhaustive_number(x: u8) {
        match (x) {
            1 => {},
            2 => {},
        }
    }

    fun unreachable_duplicate_number(x: u8) {
        match (x) {
            1 => {},
            1 => {},
            _ => {},
        }
    }
}
