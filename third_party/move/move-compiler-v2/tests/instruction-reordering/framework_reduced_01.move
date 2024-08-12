module 0xc0ffee::m {
    fun take_ref(x: &u64): u64 {
        *x
    }

    fun consume(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) {
    }

    public fun test(a: u64, b: u64, c: u64, d: u64) {
        let x = take_ref(&a);
        consume(a, x, b, c, d);
    }

}
