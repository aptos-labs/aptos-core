module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = &mut p;
        let b = p;
        let c = b;
        let d = c;
        *a = 0;
        d
    }
}
