//# publish
module 0xc0ffee::m {

    struct Test1 has copy, drop {
        a: u64,
        b: u64
    }

    struct Test2 has copy, drop {
        a: vector<u64>
    }

    struct Test3 has copy, drop {
        a: vector<Test1>
    }

    public fun test1() {
        let x = Test1 {a: 1, b: 2};
        let y = Test1 {a: 1, b: 2};

        assert!(x == y, 0);
        assert!(&x == &y, 0);

        assert!(x.a == y.a, 0);
        assert!(&x.a == &y.a, 0);

        assert!(x.b == y.b, 0);
        assert!(&x.b == &y.b, 0);
    }

    public fun test2() {
        let x = Test2 {a: vector[1, 2, 3]};
        let y = Test2 {a: vector[1, 2, 3]};

        assert!(x == y, 0);
        assert!(&x == &y, 0);

        assert!(x.a == y.a, 0);
        assert!(&x.a == &y.a, 0);

        assert!(x.a[0] == y.a[0], 0);
        assert!(&x.a[0] == &y.a[0], 0);
    }

    public fun test3() {
        let x = Test3 {a: vector[Test1 {a: 1, b:2}]};
        let y = Test3 {a: vector[Test1 {a: 1, b:2}, Test1 {a: 1, b:2}]};

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x.a != y.a, 0);
        assert!(&x.a != &y.a, 0);

        assert!(x.a[0] == y.a[0], 0);
        assert!(&x.a[0] == &y.a[0], 0);
    }


    public fun test4() {
        let x = vector[Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}];
        let y = vector[Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}];

        assert!(x == y, 0);
        assert!(&x == &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[1] == y[1], 0);
        assert!(&x[1] == &y[1], 0);

    }

    public fun test5() {
        let x = vector[Test2 {a: vector[1, 2, 3]}, Test2 {a: vector[1, 2, 3]}];
        let y = vector[Test2 {a: vector[1, 2, 3]}, Test2 {a: vector[2, 2, 2]}, Test2 {a: vector[1, 2, 3]}];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[1] != y[1], 0);
        assert!(&x[1] != &y[1], 0);
    }

    public fun test6() {
        let x = vector[Test3 {a: vector[Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}]}];
        let y = vector[Test3 {a: vector[Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}]}, Test3 {a: vector[Test1 {a: 1, b: 2}, Test1 {a: 1, b: 2}]}];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[0].a == y[0].a, 0);
        assert!(&x[0].a == &y[0].a, 0);
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6
