//# publish
module 0xc0ffee::m {
    fun foo(b: u64): u64 {
        let _x = 1;
        _x = b;
        _x
    }

    public fun test() {
        assert!(foo(2) == 2, 42);
    }
}

//# run 0xc0ffee::m::test
