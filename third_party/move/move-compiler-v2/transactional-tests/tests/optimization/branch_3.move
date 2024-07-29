//# publish
module 0xc0ffee::m {

    fun foo(b: bool, p: u64): u64 {
        let a: u64;
        if (b) {
            a = p;
        } else {
            a = p;
        };
        a
    }

    public fun test() {
        assert!(foo(true, 5) == 5, 0);
        assert!(foo(false, 5) == 5, 2);
    }
}

//# run 0xc0ffee::m::test
