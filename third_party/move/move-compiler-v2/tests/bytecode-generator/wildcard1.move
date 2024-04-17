module 0xc0ffee::m {
    fun tup(): (u64, u64) {
        (0, 0)
    }

    public fun bar() {
        let _ = tup();
    }
}
