module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun take2(_x: u64, _y: u64) {}

    public fun test(b: u64) {
        let a = one();
        take2(b, a);
    }
}
