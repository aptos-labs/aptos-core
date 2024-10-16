// See also #12301
module 0x42::m {
    // Expected to be invalid
    public fun test1(p: u64): u64 {
        let a = &mut p;
        let b = a;
        let c = b;
        *a = 0;
        *c
    }
    // Expected to be invalid
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
