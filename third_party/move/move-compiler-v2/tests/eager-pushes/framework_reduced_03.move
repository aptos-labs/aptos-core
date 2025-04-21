module 0xc0ffee::m {
    struct Wrap {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
        e: u64,
        f: u64,
    }

    struct S has key {
        x: u64,
        y: u64,
    }

    public fun make(a: u64, b: u64, c: u64, d: address, e: u64): Wrap acquires S {
        let ref = borrow_global<S>(d);
        Wrap {
            a,
            b,
            c,
            d: ref.x,
            e,
            f: ref.y,
        }
    }
}
