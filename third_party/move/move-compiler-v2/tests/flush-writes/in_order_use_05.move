module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun consume_2(_a: u64, _b: u64) {}

    public fun test() {
        let a = one();
        let b = one();
        let c = one();
        let d = one();
        consume_2(c, d);
        consume_2(a, b);
    }
}
