module 0xc0ffee::m {
    fun four(): (u64, u64, u64, u64) {
        (1, 2, 3, 4)
    }

    fun two(): (u64, u64) {
        (5, 6)
    }

    fun consume_4(_a: u64, _b: u64, _c: u64, _d: u64) {}

    fun consume_2(_a: u64, _b: u64) {}

    public fun test() {
        let (a, b, c, d) = four();
        let (e, f) = two();
        consume_4(a, b, c, d);
        consume_2(e, f);
    }
}
