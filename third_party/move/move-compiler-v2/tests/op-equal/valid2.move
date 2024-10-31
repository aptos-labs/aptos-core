module 0xc0ffee::m {
    struct S has drop {
        x: u64,
    }

    fun foo(self: &mut S): u64 {
        self.x += 1;
        1
    }

    public fun test(): u64 {
        let s = S { x: 0 };
        s.x += s.foo();
        s.x
    }
}
