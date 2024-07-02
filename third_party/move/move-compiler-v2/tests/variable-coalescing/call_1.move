module 0xc0ffee::m {
    fun id(x: u64): u64 {
        x
    }

    fun test(p: u64): u64 {
        let a = p;
        let b = p;
        let c = b;
        id(id(id(c)))
    }
}
