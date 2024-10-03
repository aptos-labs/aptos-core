//# publish
module 0xc0ffee::m1 {
    fun foo(r: &mut u64): &mut u64 {
        *r += 1;
        r
    }

    public fun test() {
        let x = 1;
        *{foo(&mut x)} += 1;
		assert!(x == 3);
    }
}

//# publish
module 0xc0ffee::m2 {
    fun foo(r: &mut u64) {
        *r += 2;
    }

    public fun test() {
        let x = 1;
        *{foo(&mut x); foo(&mut x); &mut x} += 1;
        assert!(x == 6);
    }
}

//# run --verbose -- 0xc0ffee::m1::test

//# run --verbose -- 0xc0ffee::m2::test
