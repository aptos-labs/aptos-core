module 0xc0ffee::m {
    fun sequential(p: u64): u64 {
        let a = p;
        let b = a;
        let c = b;
        let d = c;
        let e = d;
        e
    }
}
