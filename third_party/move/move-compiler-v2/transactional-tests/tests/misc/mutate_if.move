//# publish
module 0xc0ffee::m {
    struct S has copy, drop {
        x: bool,
        y: u64,
        z: u64,
    }

    fun foo1(s: S): S {
        let r = &mut (if (s.x) { s.y } else { s.z });
        *r = 2;
        s
    }

    fun foo2(s: S): S {
        *&mut (if (s.x) s.y else s.z) = 2;
        s
    }

    fun test() {
        let s1 = S { x: true, y: 1, z: 3 };
        let result1 = foo1(s1);
        assert!(s1 == result1, 0);

        let s2 = S { x: false, y: 4, z: 5 };
        let result2 = foo2(s2);
        assert!(s2 == result2, 1);
    }
}

//# run 0xc0ffee::m::test
