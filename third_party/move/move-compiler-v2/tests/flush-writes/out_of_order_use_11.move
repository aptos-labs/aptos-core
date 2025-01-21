module 0xc0ffee::m {
    fun one_one(): (u64, u64) {
        (1, 1)
    }

    fun consume_2(_a: u64, _b: u64) {}

    public fun test() {
        let (a, b) = one_one();
        let (c, d) = one_one();
        consume_2(b, c);
        consume_2(a, d);
    }
}
