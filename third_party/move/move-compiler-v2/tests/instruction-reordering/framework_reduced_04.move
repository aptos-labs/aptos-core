module 0xc0ffee::m {
    struct Foo {
        x: u64,
    }

    fun bar(x: &u64): u64 {
        *x + 1
    }

    fun baz(x: &mut u64, y: u64) {
        *x = *x + y;
    }

    public fun test(v: &mut Foo) {
        let n = bar(&v.x);
        baz(&mut v.x, n + 1);
    }

}
