/// Primitive-type match is rejected even without literals (gate is on type, not patterns).
module 0xc0ffee::primitive_match_wildcard_only {
    public fun test(x: u64): u64 {
        match (x) {
            _ => 0,
        }
    }
}
