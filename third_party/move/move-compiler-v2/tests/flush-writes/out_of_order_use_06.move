module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun multi(): (u64, u64, u64, u64, u64, u64) {
        (one(), one(), one(), one(), one(), one())
    }

    fun consume(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}

    fun test() {
        let (a, _, _, _, _, _) = multi();
        consume(one(), a, one(), one(), one(), one());
    }

}
