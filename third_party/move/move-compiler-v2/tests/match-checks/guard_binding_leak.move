module 0xc0ffee::m {
    // `y` is bound only by the first arm's pattern, not by any outer scope.
    // The catch-all `_ => y` must not see the leaked binding from the
    // guarded arm—it should produce an unbound variable error.
    fun leak(x: u64, z: u64): u64 {
        match ((x, z)) {
            (1, y) if false => y,
            _ => y,
        }
    }
}
