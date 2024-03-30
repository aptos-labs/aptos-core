module 0xc0ffee::m {
    public fun test1(p: u64): u64 {
        let a = &mut p;
        let b = a;
        let c = b;
        *a = 0;
        *c
    }

    public fun test2(p: u64): u64 {
        let a = &mut p;
        let k = &mut p;
        let b = a;
        let c = b;
        *a = 0;
        *k = 1;
        *c
    }

}
