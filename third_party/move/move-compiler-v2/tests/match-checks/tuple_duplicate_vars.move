module 0xc0ffee::m {
    // Duplicate variable name in a tuple pattern.
    fun dup_tuple_var(x: u64, y: u64): u64 {
        match ((x, y)) {
            (a, a) => a,
            _ => 0,
        }
    }
}
