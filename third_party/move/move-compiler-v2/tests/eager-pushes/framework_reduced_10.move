module 0xc0ffee::m {
    struct S {
        x: u64,
        y: u64,
        z: u64,
    }

    fun foo(l: &u64): u64 {
        *l
    }

    fun bar(r: &mut u64, i: u64) {
        *r = i;
    }

    public fun test(s: &mut S, i: u64) {
        let c = i / s.x;
        let n = foo(&s.y);
        while (c < n) {
            bar(&mut s.z, c - 1);
            c = c + 1;
        }
    }

}
