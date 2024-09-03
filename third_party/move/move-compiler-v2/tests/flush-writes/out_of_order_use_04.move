module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun consume(_a: u64, _b: u64, _c: u64) {}

    public fun test() {
        let a = one();
        let b = one();
        consume(a, b, a);
    }
}
