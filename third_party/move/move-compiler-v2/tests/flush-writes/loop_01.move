module 0xc0ffee::m {
    fun foo(x: u64): (u64, u64) {
        (x, x - 1)
    }

    public fun test1(x: u64) {
        loop {
            let y: u64;
            (y, x) = foo(x);
            if (y == 0) {
                break;
            }
        }
    }

    public fun test2(x: u64) {
        loop {
            let y: u64;
            (x, y) = foo(x);
            if (y == 0) {
                break;
            }
        }
    }
}
