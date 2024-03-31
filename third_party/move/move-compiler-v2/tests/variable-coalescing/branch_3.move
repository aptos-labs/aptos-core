module 0xc0ffee::m {

    fun test(b: bool, p: u64): u64 {
        let a: u64;
        if (b) {
            a = p;
        } else {
            a = p;
        };
        a
    }
}
