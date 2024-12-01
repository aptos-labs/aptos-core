//# publish
module 0x42::Test {

    public fun f(a: u64, b: u64): u64 {
        a * b
    }

    public inline fun quux(f:|u64, u64|u64, a: u64, b: u64): u64 {
        f(a, b)
    }

    public fun test_shadowing(): u64 {
        quux(|a, b| a - b, 10, 2)
    }
}

//# run 0x42::Test::test_shadowing
