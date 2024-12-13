module 0xc0ffee::m {
    fun foo(): (u64, u64, u64, u64, u64) {
        (1, 2, 3, 4, 5)
    }

    fun consume(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) {}

    public fun test() {
        let (a, b, c, d, e) = foo();
        consume(a, b, c, d, e);
    }
}
