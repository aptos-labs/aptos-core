module 0xc0ffee::m {

    struct S {
        a: u64,
        b: u64,
    }

    fun test(s: S): u64 {
        let p = s;
        let q = p;
        let ref = &mut p.a;
        *ref = 0;
        q.a
    }
}
