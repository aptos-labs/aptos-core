module 0xc0ffee::m {
    fun copy_kill(p: u64): u64 {
        let a = p;
        let b = a;
        p = p + 1;
        b + a
    }
}
