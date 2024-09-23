module 0xc0ffee::m {
    public fun test(x: u64) {
        let a = x;
        let b = a;
        let c = b;
        a = c;
    }

}
