module 0xc0ffee::m {
    fun consume1(_a: u64, _b: u64, _c: u64) {}

    fun one(): u64 {
        1
    }

    public fun test1(p: u64) {
        consume1(p, p, 1);
    }

    public fun test2(p: u64) {
        let q = one();
        consume1(q, p, 2);
    }

    public fun test3(p: u64) {
        let q = one();
        consume1(p, q, 3);
    }

}
