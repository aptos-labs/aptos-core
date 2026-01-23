module 0xc0ffee::m {
    public fun test(a: bool) {
        let x = if (a) {
            1
        } else {
            2
        } // semicolon is needed, should fail compilation without it
        assert!(x > 0, 1);
    }
}
