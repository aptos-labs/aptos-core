module 0xc0ffee::range_type_error {
    struct S has copy, drop { x: u64 }

    // Range on bool
    fun range_on_bool(x: bool): u64 {
        match (x) {
            true..false => 1,
            _ => 0,
        }
    }

    // Range on struct
    fun range_on_struct(x: S): u64 {
        match (x) {
            1..10 => 1,
            _ => 0,
        }
    }

    // Range on vector
    fun range_on_vector(x: vector<u8>): u64 {
        match (x) {
            1..10 => 1,
            _ => 0,
        }
    }
}
