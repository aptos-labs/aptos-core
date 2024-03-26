//# publish
module 0xc0ffee::m {
    fun foo(): u64 {
        let _y = 2;
        let x = _y;
        _y = 1;
        x
    }

    public fun test() {
        assert!(foo() == 2, 45);
    }
}

//# run 0xc0ffee::m::test
