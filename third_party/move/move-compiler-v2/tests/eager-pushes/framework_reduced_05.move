module 0xc0ffee::m {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun foo(_x: &signer, _y: u64, _z: u64) {}

    public fun test(x: &signer, y: S) {
        foo(x, y.a, y.b);
    }

}
