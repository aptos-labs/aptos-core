//# publish
module 0x42::m {
    struct S has drop{ f: u64 }
    struct R has drop{ s1: S, s2: S }

    fun t1() {
        let f;
        let s2;
        R { s1: S { f }, s2 } = &R { s1: S{f: 0}, s2: S{f: 1} };
        assert!(*f == 0, 0);
        assert!(s2.f == 1, 1);
    }

    fun t2() {
        let f;
        let s2;
        R { s1: S { f }, s2 } = &mut R { s1: S{f: 0}, s2: S{f: 1} }; f; s2;
        assert!(*f == 0, 0);
        assert!(s2.f == 1, 1);
        f = &mut 5;
        s2 = &mut S { f: 0 };
        assert!(*f == 5, 0);
        assert!(s2.f == 0, 1);
    }
}

//# run 0x42::m::t1

//# run 0x42::m::t2
