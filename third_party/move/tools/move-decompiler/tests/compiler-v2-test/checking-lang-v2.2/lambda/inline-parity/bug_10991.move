//# publish
module 0x42::Test {
    fun foo(f:|u64, u64| u64, g: |u64, u64| u64, x: u64, _y: u64): u64 {
        f(x, _y) + g(x, _y)
    }

    public fun test() {
        assert!(foo(|x, _| x, |_, y| y, 10, 100) == 110, 0);
    }
}

//# run 0x42::Test::test
