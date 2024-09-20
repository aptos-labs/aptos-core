module 0x42::Test {
    inline fun foo(f:|u64| u64, x: u64): u64 {
        f(x)
    }

    public fun test(): u64 {
        foo(|_: u64| 3, 10)
    }
}
