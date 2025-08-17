//# publish
module 0x42::Test {
    fun foo(f:|u64| u64, g: |u64| u64, x: u64, _: u64): u64 {
        f(x) + g(x)
    }

    public fun test() {
        assert!(foo(|_| 3, |_| 10, 10, 100) == 13, 0);
    }
}

//# run 0x42::Test::test
