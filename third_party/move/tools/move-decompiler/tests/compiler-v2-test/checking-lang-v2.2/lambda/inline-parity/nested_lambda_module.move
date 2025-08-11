//# publish
module 0x42::Test1 {
    public fun apply(f: |u64, u64|u64, x: u64, y: u64): u64 {
        f(x, y)
    }
}

//# publish
module 0x42::Test {
    use 0x42::Test1;

    public fun test(): u64 {
        Test1::apply(|x, y| x + y, 1, Test1::apply(|x, y| x * y, 2, 1))
    }
}

//# run 0x42::Test::test
