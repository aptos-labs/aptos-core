module 0x42::m {
    // A loop invariant may be given in the loop header OR in a trailing spec
    // block after the body, but not both.
    fun both(n: u64): u64 {
        let s = 0;
        for (i in 0..n spec { invariant s <= i; }) {
            s = s + i;
        } spec { invariant s == i * (i - 1) / 2; };
        s
    }
}
