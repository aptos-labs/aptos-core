module 0xc0ffee::m {
    fun one_one(): (u64, u64) {
        (1, 1)
    }

    fun take1(_x: u64) {}

    fun take2(_x: u64, _y: u64) {}

    public fun test() {
        let (a, b) = one_one();
        take1(a);
        take1(b);
    }
}
