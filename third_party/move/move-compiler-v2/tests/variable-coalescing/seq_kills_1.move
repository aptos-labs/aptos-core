module 0xc0ffee::m {
    fun test(p: u64): bool {
        let a = p;
        let b = a;
        let c = b;

        b = p + 1; // kill b := a, which removes the whole copy chain
        a == c
    }
}
