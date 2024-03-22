module 0xc0ffee::m {
    public fun test(p: bool): u32 {
        let x = 1;
        let y = x;
        if (p) {
            y = y;
            y = y;
            y
        } else {
            y = y;
            9
        }
    }
}
