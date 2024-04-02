module 0xc0ffee::m {
    fun update(p: &mut u64) {
        *p = 0;
    }

    fun test(p: u64): u64 {
        let a = p;
        let b = p;
        let c = b;
        update(&mut a);
        c
    }
}
