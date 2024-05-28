//# publish
module 0xc0ffee::m {
    fun test() {
        let x = 5;
        let x_ref = &x;
        x_ref;
        x_ref = &x;
        assert!(*x_ref == 5, 0);
    }
}

//# run 0xc0ffee::m::test
