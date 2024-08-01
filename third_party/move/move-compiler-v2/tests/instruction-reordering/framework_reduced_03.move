module 0xc0ffee::m {
    fun compute(x: u64): u64 {
        x + 1
    }

    fun take1(x: &u64): u64 { *x }

    fun take2(_x: &u64, _y: u64) {}

    public fun test(x: u64) {
        let k = compute(x);
        take2(&x, take1(&k));
    }
}
