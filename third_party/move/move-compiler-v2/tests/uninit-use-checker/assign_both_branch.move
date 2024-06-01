module 0xc0ffee::m {
    fun test(cond: bool): u64 {
        let x: u64;
        if (cond) {
            x = 1;
        } else {
            x = 2;
        };
        x
    }

}
