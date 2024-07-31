module 0xc0ffee::m {
    public fun consume(_a: u64, _b: u64, _c: u64, _d: u64) {
    }

    public fun test(x: u64) {
        consume(x, x, 1, x);
    }
}
