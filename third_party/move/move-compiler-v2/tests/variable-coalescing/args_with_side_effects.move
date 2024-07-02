module 0xc0ffee::m {
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    public fun test(p: u64): u64 {
        add(p, {p = p + 1; p})
    }

}
