//# publish
module 0x42::OtherModule {
    public fun g(a: u64, b: u64): u64 {
        a + b
    }

    public fun h(a: u64, b: u64): u64 {
        2 * a + b
    }
}

//# publish
module 0x42::Test {
    use 0x42::OtherModule::g;

    public fun f(a: u64, b: u64): u64 {
        a * b
    }

    public inline fun quux(f:|u64, u64|u64, g:|u64|u64, i:|u8|u8, a: u64, b: u64): u64 {
        use 0x42::OtherModule::h;
        f(a, b) * g(a, b) * h(a, b)
    }

    public fun test_shadowing(): u64 {
        quux(|a, b| a - b, |a| a + 2, |b| 255u8-b, 10, 2)
    }
}

//# run 0x42::Test::test_shadowing
