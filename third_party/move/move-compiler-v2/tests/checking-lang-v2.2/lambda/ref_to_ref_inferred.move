module 0xc0ffee::m {
    fun test1() {
        let f = |x| {
            **x + 1;
            (x, x)
        };
    }

    fun test2() {
        let f = |x, y| {
            let p = *x;
            let q = *p + y;
            x
        };
    }

    fun test3() {
        let f = |x, y| {
            ******x + ***y
        };
        let g = f;
    }
}
