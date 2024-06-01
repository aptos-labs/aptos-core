module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = p;
        let b = &mut p;
        *b = 1;
        a
    }
}
