module 0xc0ffee::m {
    struct Wrap has drop, key {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
    }

    fun zero(_y: u64): u64 {
        0
    }

    fun bar(_x: &signer, _y: &mut u64, _w: Wrap) {}

    public fun test(x: signer, a: address) acquires Wrap {
        let y = 0;
        let ref = borrow_global_mut<Wrap>(a);
        bar(&x, &mut ref.a, Wrap {a: y, b: 0, c: zero(y), d: 0});
    }
}
