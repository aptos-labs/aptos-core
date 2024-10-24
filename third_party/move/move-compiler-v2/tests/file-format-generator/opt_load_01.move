module 0xc0ffee::m {
    fun consume(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}

    public fun test01(a: u64) {
        consume(a, 1, 2, 3, 4, 5);
    }

    public fun test02(a: u64) {
        consume(a, 1, 2, a, 4, 5);
    }

    public fun test03(a: u64) {
        consume(a, 1, 2, 3, 4, 5);
        consume(a, 1, 2, 3, 4, 5);
    }
}
