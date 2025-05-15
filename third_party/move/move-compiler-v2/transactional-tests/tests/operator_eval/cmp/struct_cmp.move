//# publish
module 0xc0ffee::m {

    struct Test has copy, drop {
        a: u64,
        b: u64
    }

    struct Test1 has copy, drop {
        a: Test,
        b: u64
    }

    public fun test1() {
        let x = Test {a: 1, b: 2};
        let y = Test {a: 1, b: 2};
        assert!(x == y, 0);
        assert!(&x == &y, 0);
    }

    public fun test2() {
        let x = Test {a: 2, b: 2};
        let y = Test {a: 1, b: 2};
        assert!(&x != &y, 0);
    }

    public fun test3() {
        let x = Test1 {
            a: Test {a: 1, b: 2},
            b: 2
        };

        let y = Test1 {
            a: Test {a: 1, b: 2},
            b: 2
        };

        assert!(x == y, 0);
        assert!(&x == &y, 0);
    }

     public fun test4() {
        let x = Test1 {
            a: Test {a: 1, b: 2},
            b: 1
        };

        let y = Test1 {
            a: Test {a: 1, b: 2},
            b: 2
        };

        assert!(x != y, 0);
        assert!(&x != &y, 0);
    }

    public fun test5() {
        let x = Test1 {
            a: Test {a: 1, b: 2},
            b: 1
        };

        let y = Test1 {
            a: Test {a: 1, b: 2},
            b: 2
        };

        assert!(x.a == y.a, 0);
        assert!(&x.a == &y.a, 0);
    }

    public fun test6() {
        let x = Test1 {
            a: Test {a: 1, b: 6},
            b: 1
        };

        let y = Test1 {
            a: Test {a: 1, b: 2},
            b: 2
        };

        assert!(x.a != y.a, 0);
        assert!(&x.a != &y.a, 0);
    }

}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6
