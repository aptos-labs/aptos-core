module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun consume_4(_a: u64, _b: u64, _c: u64, _d: u64) {}

    fun consume_2(_a: u64, _b: u64) {}

    public fun test() {
        let a = one();
        let b = one();
        let c = one();
        let d = one();
        let e = one();
        let f = one();
        consume_2(e, f);
        consume_4(a, b, c, d);
    }
}
