module 0xc0ffee::m {
    struct S has key {
        i: u64,
    }

    public fun test(): S {
        let s = S { i: 1 };
        let f = || s;
        f()
    }
}
