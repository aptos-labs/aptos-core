module 0xc0ffee::m {
    fun test(): u64 {
        let t = 1;
        let u = 2;
        t = t + 1;
        let b = u;
        b
    }
}
