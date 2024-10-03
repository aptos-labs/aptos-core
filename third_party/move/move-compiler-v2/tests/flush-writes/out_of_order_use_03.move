module 0xc0ffee::m {
    fun one_one(): (u64, u64) {
        (1, 1)
    }

    fun consume(_a: u64, _b: u64, _c: u64) {}

    public fun test() {
        let (a, b) = one_one();
        consume(b, a, a);
    }
}
