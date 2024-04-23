module 0xc0ffee::m {
    struct Foo has copy {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
        e: u64,
    }

    fun sequential(p: Foo): Foo {
        let a = p;
        let b = a;
        let c = b;
        let d = c;
        let e = d;
        e
    }
}
