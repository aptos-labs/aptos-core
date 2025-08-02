module 0x42::m {
    fun f(r: &mut u64): &mut u64 {
        r
    }
    fun g() {
        let v = 22;
        let r = &mut v;
        r = f(r);
        let _r = &v;
    }
}
