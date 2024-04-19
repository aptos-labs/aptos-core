//# publish
module 0xc0ffee::m {

    fun foo(b: bool, p: u64, q: u64): u64 {
        let a: u64;
        if (b) {
            a = p;
        } else {
            a = q;
        };
        a
    }

    public fun test() {
        assert!(foo(true, 4, 5) == 4, 0);
        assert!(foo(false, 4, 5) == 5, 0);
    }
}

//# run 0xc0ffee::m::test
