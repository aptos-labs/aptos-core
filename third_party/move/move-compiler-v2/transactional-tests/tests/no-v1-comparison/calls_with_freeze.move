//# publish
module 0x42::m {

    struct S has drop { x: u64 }

    fun sum(self: &S, other: &S): u64 { self.x + other.x }

    fun test_arg_freeze(s: S) : u64 {
        let p1m = &mut s;
        let s2 = S { x: 4 };
        let p2m = &mut s2;
        let x1 = p1m.sum(p2m);
        let x2 = p1m.sum(p1m);
        let x3 = (&s).sum(p2m);
        x1 + x2 + x3
    }

    fun test(): u64 {
        test_arg_freeze(S{x: 2})
    }
}

//# run 0x42::m::test
