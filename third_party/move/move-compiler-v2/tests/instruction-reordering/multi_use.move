module 0xc0ffee::m {
    fun multi_consume(_a: u64, _b: u64, _c: u64, _d: u64) {}

    fun consume(_a: u64, _b: u64) {}

    fun one(): u64 {
        1
    }    

    public fun test1() {
        let x = one();
        multi_consume(x, 2, x, 1);
    }

    public fun test2(x: u64) {
        multi_consume(x, 2, x, 1);
    }

    public fun test3() {
        let x = one();
        consume(x, 2);
        consume(x, 1);
    }

    public fun test4(x: u64) {
        consume(x, 2);
        consume(x, 1);
    }

}
