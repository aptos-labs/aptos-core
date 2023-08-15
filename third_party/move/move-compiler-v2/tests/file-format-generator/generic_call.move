module 0x42::Test {
    fun identity<T>(x: T): T {
        x
    }

    fun foo(x: u64): u64 {
        identity(x)
    }
}
