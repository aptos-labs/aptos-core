module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun bar(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}

    public fun test(x: u64) {
        bar(x, one(), one(), one(), one(), one());
    }
}
