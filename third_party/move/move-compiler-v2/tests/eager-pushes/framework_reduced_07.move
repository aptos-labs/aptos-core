module 0xc0ffee::m {
    struct S {
        x: u64,
        y: u64,
    }

    fun foo(l: &mut u64, i: u64): &mut u64 {
        *l = i;
        l
    }

    fun bar(r: &mut u64, i: u64) {
        *r = i;
    }

    public fun test(s: &mut S, i: u64) {
        bar(foo(&mut s.x, i / s.y), i / s.y);
    }
}
