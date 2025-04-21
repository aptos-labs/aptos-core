module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun consume_1(_a: u64) {}

    fun consume_2(_a: u64, _b: u64) {}

    public fun test() {
        let a = one();
        let b = one();
        let c = one();
        consume_2(a, b);
        consume_1(c);
    }
}
