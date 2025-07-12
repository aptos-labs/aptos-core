module 0xc0ffee::m {
    inline fun another(f: ||u64): u64 {
        f()
    }

    public inline fun apply(f: ||u64): u64 {
        another(f)
    }
}
