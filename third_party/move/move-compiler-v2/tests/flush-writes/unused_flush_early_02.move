module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun bar() {}

    public fun test(): (u64, u64) {
        let x = one();
        let y = one();
        let z = one();
        if (y == 0) {
            bar();
        };
        (x, z)
    }

}
