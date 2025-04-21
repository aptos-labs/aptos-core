module 0xc0ffee::m {
    struct Wrap {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
        e: u64,
    }

    struct S {
        x: u64,
    }

    public fun make(a: u64, b: u64, c: u64, d: &S, e: u64): Wrap {
        Wrap {
            a,
            b,
            c,
            d: d.x,
            e,
        }
    }
}
