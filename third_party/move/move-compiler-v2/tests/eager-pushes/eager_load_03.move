module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun bar(_x: &mut u64) {}

    fun baz(_x: u64, _y: u64) {}

    public fun foo(x: u64) {
        let t = one();
        bar(&mut x);
        baz(x, t);
    }
}
