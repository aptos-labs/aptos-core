module 0xc0ffee::m {
    fun test(p: u64): bool {
        let a = p;
        let b = a;
        let c = b;

        a = p + 1; // kill b := a
        b == c
    }
}
