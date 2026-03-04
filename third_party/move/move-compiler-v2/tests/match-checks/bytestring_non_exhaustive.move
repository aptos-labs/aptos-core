module 0xc0ffee::m {
    // Non-exhaustive byte string match: no wildcard catch-all.
    fun test(v: vector<u8>): u64 {
        match (v) {
            b"hello" => 1,
            b"world" => 2,
        }
    }
}
