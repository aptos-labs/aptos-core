//# publish
module 0xc0ffee::m {
    public struct S has copy, drop {
        x: T,
    }

    public struct T has copy, drop {
        y: u64,
    }
}

//# publish
module 0xc0ffee::test_m {
    use 0xc0ffee::m::S;
    use 0xc0ffee::m::T;

    fun foo(s: S, p: S): S {
        *&mut {s.x.y = p.x.y; s.x.y} = 1;
        s
    }

    public fun test() {
        let s = S { x: T { y: 42 } };
        let p = S { x: T { y: 43 } };
        let result = foo(s, p);
        assert!(result.x.y == 43, 0);
    }
}

//# run 0xc0ffee::test_m::test
