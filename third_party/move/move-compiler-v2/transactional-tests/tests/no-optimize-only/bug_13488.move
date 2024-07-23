//# publish
module 0xc0ffee::m {

    fun test() {
        let x = 0;
        let r1 = &x;
        let r2 = &x;
        x + copy x;
        r1;
        r2;
        assert!(*r1 == 0, 1);
        assert!(*r2 == 0, 1);
    }

    fun test2() {
        let x = 2;
        let _r1 = &x;
        let _r2 = &x;
        assert!(x + copy x == 4, 1);
        _r1;
        _r2;
    }

}

//# run 0xc0ffee::m::test

//# run 0xc0ffee::m::test2
