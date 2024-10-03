//# publish
module 0x42::Test {

    public inline fun apply(f: |u64, u64|u64, x: u64, y: u64): u64 {
        f(x, y)
    }

    public fun test(): u64 {
        apply(|x: u64, y: u64| x + y, 1, 2)
    }
}

//# run 0x42::Test::test
