module 0x42::m {

    struct S { x: u64 }

    fun sum(self: &S, _other: &S): u64 { abort 1 }

    fun test_arg_freeze(s: S) : u64 {
        let p1 = &s;
        let p1m = &mut s;
        let s2 = S { x: 4 };
        let p2 = &s2;
        let p2m = &mut s;
        let x1 = p1m.sum(p1);
        let x2 = p1m.sum(p1m);
        let x3 = p1m.sum(p2);
        let x4 = p2m.sum(p2m);
        x1 + x2 + x3 + x4
    }

}
