module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = &mut p;
        let b = a;
        let c = b;
        *a = 0;
        *c
    }
}
