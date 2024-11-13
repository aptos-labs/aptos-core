module 0xc0ffee::m {
    struct S {
        x: u64,
    }

    fun bar(r: &mut S, i: u64): u64 {
        r.x = i;
        i
    }

    fun foo(l: &mut S, i: u64) {
        l.x = i;
    }

    fun destroy(s: S) {
        let S { x: _ } = s;
    }

    public fun test(l: &mut S, r: S) {
        let i = 0;
        while (i < 42) {
            foo(l, bar(&mut r, i));
            i = i + 1;
        };
        destroy(r);
    }
}
