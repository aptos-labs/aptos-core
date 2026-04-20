module 0xc0ffee::range_unbounded_error {
    // Bare `..` should be rejected — use `_` instead.
    fun bare_dotdot(x: u8): u64 {
        match (x) {
            .. => 1,
        }
    }
}
