module 0xc0ffee::m {
    struct S {}

    public fun foo(s: S) {
        let _ = s;
    }

    public fun bar() {
        let s = S{};
        let _ = s;
    }
}
