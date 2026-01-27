module 0x123::a {
    inline fun foo(x: bool) {
        if (!x) {
            assert!(1 == 1);
        }
    }

    public fun bar() {
        foo(false);
    }
}
