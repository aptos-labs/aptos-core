//# publish
module 0x42::Test {
    fun foo(f:|u64, u64| u64, g: |u64, u64| u64,
	h:|u64, u64| u64, i: |u64, u64| u64,
	x: u64, y: u64): u64 {
            f(x, y) + g(x, y) + h(x, y) + i(x, y)
    }

    public fun test() {
        assert!(foo(|x, _| x, |_, y| y,
	    |a, _b| a, |_c, d| d,
	    10, 100) == 220, 0);
    }
}

//# run 0x42::Test::test
