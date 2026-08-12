module 0x42::m {
    // A header spec block sits inside the loop header, before the body, so a
    // `for` loop with a header spec and a non-block body ends with the body
    // (not `}`). A semicolon is therefore still required before the following
    // statement, just as for a plain non-block-bodied `for` loop.
    fun header_spec_nonblock_body(n: u64): u64 {
        let s = 0;
        for (i in 0..n spec { invariant s <= n * n; }) s = s + i
        s
    }
}
