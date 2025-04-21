module 0xc0ffee::m {
    fun one_one(): (u64, u64) {
        (1, 1)
    }

    fun consume_3(_a: u64, _b: u64, _c: u64) {}

    fun consume_1(_a: u64) {}

    public fun test() {
        let (a, b) = one_one();
        let (c, d) = one_one();
        consume_3(a, c, d);
        consume_1(b);
    }
}
