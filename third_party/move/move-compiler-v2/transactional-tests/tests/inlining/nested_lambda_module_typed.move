//# publish
module 0x42::Test1 {
    public inline fun apply(f: |u64, u64|u64, x: u64, y: u64): u64 {
        f(x, y)
    }
}

//# publish
module 0x42::Test {
    use 0x42::Test1;

    public fun test(): u64 {
        Test1::apply(|x: u64, y: u64| x + y, 1, Test1::apply(|x: u64, y: u64| x * y, 2, 1))
    }
}

//# run 0x42::Test::test
