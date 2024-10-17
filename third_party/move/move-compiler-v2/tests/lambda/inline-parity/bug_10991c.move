//# publish
module 0x42::Test {
    fun foo(g: |u64, u64, u64, u64| u64, x: u64, y: u64, z: u64, q:u64): u64 {
        g(x, y, z, q)
    }

    public fun test() {
        assert!(foo(|_, y, _, q| y + q,
	    10, 100, 1000, 10000) == 10100, 0);
    }
}

//# run 0x42::Test::test
