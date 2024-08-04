module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    public fun test1(): (u64, u64) {
        let _x = one();
        let y = one();
        let z = one();
        (y, z)
    }

    public fun test2(): (u64, u64) {
        let x = one();
        let _y = one();
        let z = one();
        (x, z)
    }

    public fun test3(): (u64, u64) {
        let x = one();
        let _y = one();
        let z = one();
        (z, x)
    }
}
