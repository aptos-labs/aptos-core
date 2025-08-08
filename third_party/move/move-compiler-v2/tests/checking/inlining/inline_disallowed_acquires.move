module 0xc0ffee::m {
    struct R has key {
        x: u64
    }

    inline fun test() acquires R {}
}
