module 0x42::Test {

    public inline fun f(): u64 {
        g()
    }

    public inline fun g(): u64 {
        h()
    }

    public inline fun h(): u64 {
        f()
    }

    public fun test(): u64 {
        f()
    }
}
