//# publish
module 0xc0ffee::m {
    public fun foo(): u64 {
        let _t = 1;
        _t = 2;
        _t
    }

    public fun check() {
        assert!(foo() == 2, 0);
    }
}

//# run 0xc0ffee::m::check
