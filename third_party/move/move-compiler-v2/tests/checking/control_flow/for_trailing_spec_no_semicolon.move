module 0x42::m {
    // A `for` loop with a non-block body and a trailing spec block ends with
    // `}`, so (like a block-bodied loop) no semicolon is required before the
    // following statement.
    fun trailing_spec_nonblock_body(n: u64): u64 {
        let s = 0;
        for (i in 0..n) s = s + i spec { invariant s <= n * n; }
        s
    }
}
