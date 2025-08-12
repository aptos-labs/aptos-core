//# publish
module 0xc0ffee::m {
    struct S has copy, drop {
        x: u64
    }

    public fun test(): u64 {
        let s = make();
        s.foo()
    }

    fun foo(self: &mut S): u64 {
        make().bar(|| {
            // inline function captures param
            mod(&mut self.x);
        });
        self.x
    }

    fun mod(x: &mut u64) {
        *x = 42;
    }

    inline fun make(): S {
        S { x: 0 }
    }

    inline fun bar(self: &mut S, f: ||) {
        self.x = 12;
        f()
    }
}

//# run 0xc0ffee::m::test
