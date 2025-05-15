//# publish
module 0xc0ffee::m {


    public fun test1() {
        let x = vector[1, 2, 3];
        let y = vector[1, 2, 3];

        assert!(x == y, 0);
        assert!(&x == &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[0] != y[1], 0);
        assert!(&x[0] != &y[1], 0);

    }

    public fun test2() {
        let x = vector[1, 1, 1];
        let y = vector[2, 2, 2];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] != y[0], 0);
        assert!(&x[0] != &y[0], 0);

        assert!(x[0] != y[1], 0);
        assert!(&x[0] != &y[1], 0);

    }

    public fun test3() {
        let x = vector[vector[1, 2, 3]];
        let y = vector[vector[1, 2, 3]];

        assert!(x == y, 0);
        assert!(&x == &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[0][0] == y[0][0], 0);
        assert!(&x[0][0] == &y[0][0], 0);
    }

    public fun test4() {
        let x = vector[vector[1, 1, 1]];
        let y = vector[vector[2, 2, 2]];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] != y[0], 0);
        assert!(&x[0] != &y[0], 0);

        assert!(x[0][0] != y[0][0], 0);
        assert!(&x[0][0] != &y[0][0], 0);
    }

    public fun test5() {
        let x = vector[vector[1, 1, 1]];
        let y = vector[vector[1, 1, 1], vector[2, 2, 2]];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] == y[0], 0);
        assert!(&x[0] == &y[0], 0);

        assert!(x[0][0] == y[0][0], 0);
        assert!(&x[0][0] == &y[0][0], 0);
    }

    public fun test6() {
        let x = vector[vector[1, 1, 1, 1, 1, 1]];
        let y = vector[vector[1, 1, 1]];

        assert!(x != y, 0);
        assert!(&x != &y, 0);

        assert!(x[0] != y[0], 0);
        assert!(&x[0] != &y[0], 0);

        assert!(x[0][0] == y[0][0], 0);
        assert!(&x[0][0] == &y[0][0], 0);

        assert!(x[0][0] == y[0][1], 0);
        assert!(&x[0][0] == &y[0][1], 0);
    }

}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6
