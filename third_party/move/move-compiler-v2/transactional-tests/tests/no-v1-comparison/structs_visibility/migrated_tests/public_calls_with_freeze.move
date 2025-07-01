//# publish
module 0x42::m {
    public struct S has drop { x: u64 }
}

//# publish
module 0x42::m_calls_with_freeze {
    use 0x42::m::S;

    fun sum(s: &S, other: &S): u64 { s.x + other.x }

    fun test_arg_freeze(s: S) : u64 {
        let p1m = &mut s;
        let s2 = S { x: 4 };
        let p2m = &mut s2;
        let x1 = sum(p1m, p2m);
        let x2 = sum(p1m, p1m);
        let x3 = sum(&s, p2m);
        x1 + x2 + x3
    }

    fun test(): u64 {
        test_arg_freeze(S{x: 2})
    }
}

//# run 0x42::m_calls_with_freeze::test
