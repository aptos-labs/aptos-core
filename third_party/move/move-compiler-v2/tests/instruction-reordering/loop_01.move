module 0xc0ffee::m {
    fun foo(x: u64): u64 {
        x + 1
    }

    public fun test1(x: u64) {
        loop {
            x = foo(x);
            if (x > 10) {
                break;
            }
        }
    }

    public fun test2(x: u64, y: u64, z: u64): u64 {
        while (y < z) {
            x = foo(y);
            y = x;
        };
        x + y
    }

}
