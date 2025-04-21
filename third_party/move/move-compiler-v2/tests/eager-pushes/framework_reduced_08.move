module 0xc0ffee::m {
    struct S {
        x: u64,
        y: u64,
    }

    fun foo(l: &u64, _i: u64): &u64 {
        l
    }

    fun bar(_r: &u64, _i: u64) {}

    public fun test(s: &S, i: u64) {
        bar(foo(&s.x, i / s.y), i / s.y);
    }
}
