/// Range patterns in match arms are rejected before the primitive-match language version.
module 0xc0ffee::primitive_match_range {
    public fun test(x: u64): u64 {
        match (x) {
            1..10 => 1,
            _ => 0,
        }
    }
}
