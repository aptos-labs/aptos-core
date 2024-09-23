module 0xc0ffee::m {
    public fun test(x: u64, y: u64): (u64, u64) {
        let i = x;
        let t;
        while (i > 0) {
            t = x;
            x = y;
            y = t;
            i = i - 1;
        };
        (x, y)
    }

}
