module 0xc0ffee::m {
    public fun test(p: bool): u64 {
        if (p) {
            abort 0
        } else {
            return 1
        };
        42
    }
}
