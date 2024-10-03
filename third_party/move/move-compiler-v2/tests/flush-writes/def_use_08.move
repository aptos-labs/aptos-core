module 0xc0ffee::m {
    fun foo(): (u64, u64, u64, u64) {
        (1, 2, 3, 4)
    }

    fun take(_x: u64, _y: u64, _z: u64, _w: u64) {}

    public fun test() {
        let (a, b, c, d) = foo();
        take(b, c, d, a);
    }
}
