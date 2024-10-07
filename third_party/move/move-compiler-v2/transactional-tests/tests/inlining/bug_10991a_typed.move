//# publish
module 0x42::Test {
    inline fun foo(f:|u64, u64| u64, g: |u64, u64| u64,
	h:|u64, u64| u64, i: |u64, u64| u64,
	x: u64, y: u64): u64 {
            f(x, y) + g(x, y) + h(x, y) + i(x, y)
    }

    public fun test() {
        assert!(foo(|x: u64, _: u64| x, |_: u64, y: u64| y,
	    |a: u64, _b: u64| a, |_c: u64, d: u64| d,
	    10, 100) == 220, 0);
    }
}

//# run 0x42::Test::test
