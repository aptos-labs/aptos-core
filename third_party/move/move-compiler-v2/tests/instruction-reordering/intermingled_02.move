module 0xc0ffee::m {
    fun foo(_: &u64) {}

    public fun test(p: u64, q: u64) {
        let x = &p;
        let y = &q;
        foo(x);
        foo(y);
    }

}
