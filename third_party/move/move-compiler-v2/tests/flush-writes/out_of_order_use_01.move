module 0xc0ffee::m {
    public fun one(): u64 {
        1
    }

    public fun consume(
        _a: u64,
        _b: u64,
        _c: u64,
        _d: u64,
        _e: u64,
        _f: u64,
        _g: u64
    ) {}

    public fun test() {
        let a = one();
        let b = one();
        let c = one();
        let d = one();
        let e = one();
        let f = one();
        let g = one();
        consume(b, c, d, e, f, g, a);
    }
}
