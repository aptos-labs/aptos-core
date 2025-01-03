module 0xc0ffee::m {
    fun bar(_x: &mut u64) {}

    fun baz(_x: u64, _y: u64, _z: u64) {}

    public fun foo(x: u64) {
        bar(&mut x);
        baz(x, 1, 2);
    }
}
