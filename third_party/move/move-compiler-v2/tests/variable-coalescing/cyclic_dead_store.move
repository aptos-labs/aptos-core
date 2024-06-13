module 0xc0ffee::m {
    public fun test1(x: u64, a: u64, b: u64) {
        let i = 0;
        while (i < x) {
            a = b;
            b = a;
            i = i + 1;
        }
    }

    public fun test2(x: u64, a: u64) {
        let i = 0;
        while (i < x) {
            a = a;
            i = i + 1;
        }
    }

}
