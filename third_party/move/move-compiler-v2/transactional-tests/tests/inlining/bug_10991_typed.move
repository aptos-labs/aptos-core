//# publish
module 0x42::Test {
    inline fun foo(f:|u64, u64| u64, g: |u64, u64| u64, x: u64, _y: u64): u64 {
        f(x, _y) + g(x, _y)
    }

    public fun test() {
        assert!(foo(|x: u64, _: u64| x, |_: u64, y: u64| y, 10, 100) == 110, 0);
    }
}

//# run 0x42::Test::test
