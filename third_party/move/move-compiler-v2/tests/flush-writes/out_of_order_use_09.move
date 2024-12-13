module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun consume_5(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) {}

    fun consume_1(_a: u64) {}

    public fun test() {
        let a = one();
        let b = one();
        let c = one();
        let d = one();
        let e = one();
        let f = one();
        consume_5(a, c, d, e, f);
        consume_1(b);
    }
}
