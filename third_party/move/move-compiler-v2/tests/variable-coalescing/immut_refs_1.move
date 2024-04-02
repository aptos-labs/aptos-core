module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = &p;
        let b = a;
        let c = b;
        *c
    }
}
