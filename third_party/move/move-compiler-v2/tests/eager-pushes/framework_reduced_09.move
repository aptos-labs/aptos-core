module 0xc0ffee::m {
    struct S {
        x: u64,
        y: u64,
    }

    fun foo(l: &u64): u64 {
        *l
    }

    fun bar(r: &mut u64, i: u64): &mut u64 {
        *r = i;
        r
    }

    fun baz(r: &mut u64, i: u64) {
        *r = i;
    }

    public fun test(s: &mut S, v: u64) {
        let n = foo(&s.x);
        if (s.x == n * s.y) {
            baz(bar(&mut s.x, n), v);
        }
    }
}
