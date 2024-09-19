module 0xc0ffee::m {
    public fun test(x: u64) {
        let y = &mut x;
        *y = 42;
    }

}
