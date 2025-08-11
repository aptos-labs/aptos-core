//# publish
module 0xc0ffee::m {
    fun add3(a: u8, b: u8, c: u8): u8 {
        a + b + c
    }

    public fun test() {
        let f1 = || add3(0, 1, 2);
        assert!(f1() == 3);

        let f2 = |x| add3(x, 0, 1);
        assert!(f2(2) == 3);

        let f3 = |x| add3(0, x, 1);
        assert!(f3(2) == 3);

        let f4 = |x| add3(0, 1, x);
        assert!(f4(2) == 3);

        let f5 = |x, y| add3(0, x, y);
        assert!(f5(2, 1) == 3);

        let f6 = |x, y| add3(x, y, 0);
        assert!(f6(2, 1) == 3);

        let f7 = |x, y| add3(x, 0, y);
        assert!(f7(2, 1) == 3);

        let f8 = |x, y, z| add3(x, y, z);
        assert!(f8(2, 0, 1) == 3);

        let f9 = |x| add3(x, 0, x);
        assert!(f9(2) == 4);
    }
}

//# run 0xc0ffee::m::test
