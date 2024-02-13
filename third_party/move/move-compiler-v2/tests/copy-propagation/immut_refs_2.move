module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = &p;
        let b = p;
        let c = b;
        let d = c;
        d
    }
}
