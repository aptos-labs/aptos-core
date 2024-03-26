//# publish
module 0xc0ffee::m {
    fun foo(n: u64): u64 {
        let _y = 2;
        let x = _y;
        let i = 0;
        while (i < n) {
            _y = 1;
            i = i + 1;
            x = 3;
        };
        x
    }

    public fun test() {
        assert!(foo(0) == 2, 55);
        assert!(foo(1) == 3, 56);
    }
}

//# run 0xc0ffee::m::test
