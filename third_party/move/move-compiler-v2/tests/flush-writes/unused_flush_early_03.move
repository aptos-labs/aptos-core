module 0xc0ffee::m {
    fun foo(): (u64, u64, u64) {
        (1, 2, 3)
    }

    fun bar() {}

    public fun test1() {
        let (x, y, z) = foo();
        if (x == 0) {
            bar();
        };
        if (y == 0) {
            bar();
        };
        if (z == 0) {
            bar();
        };
    }

}
