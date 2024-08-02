module 0xc0ffee::m {
    public fun test2(): u64 {
        let x = 40;
        let (y, _) = (move x, x);
        y
    }
}
