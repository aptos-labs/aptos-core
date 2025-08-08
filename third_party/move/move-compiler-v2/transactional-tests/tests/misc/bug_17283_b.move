//# publish
module 0xc0ffee::m {
    struct S has copy, drop {
        x: u64
    }

    public fun test(): u64 {
        let s = make();
        foo(&mut s)
    }

    fun foo(s: &mut S): u64 {
        bar(make(), || {
            // inline function captures param
            s.x = 43;
            mod(&mut s.x);
        });
        s.x
    }

    fun mod(x: &mut u64) {
        *x = 42;
    }

    inline fun make(): S {
        S { x: 0 }
    }

    inline fun bar(s: S, f: ||) {
        s.x = 12;
        f()
    }
}

//# run 0xc0ffee::m::test
