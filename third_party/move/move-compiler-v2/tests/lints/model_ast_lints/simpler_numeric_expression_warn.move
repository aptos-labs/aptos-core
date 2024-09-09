module 0xc0ffee::m {
    public fun test1(x: u64): u64 {
        (x & 0) + 1
    }

    public fun test2(x: u64): u64 {
        (0 & x) * 0
    }

    public fun test3(x: u64): u64 {
        ((0 * x) % 1) | 0
    }

    public fun test4(x: u64): u64 {
        (x ^ 0) - 0 + (x >> 0) + (x << 0) + 0
    }

    public fun test5(x: u64): u64 {
        (x / 1) * 1
    }

    public fun test6(x: u64): u64 {
        0 + (0 | x) + (0 ^ x)
    }

    public fun test7(x: u64): u64 {
        1 * x
    }

    public fun test8(x: u8): u64 {
        0 >> x + 0 << x
    }
}

#[lint::skip(simpler_numeric_expression)]
module 0xc0ffee::no_warn1 {
    public fun test8(x: u8): u64 {
        0 >> x + 0 << x
    }
}

module 0xc0ffee::no_warn2 {
    #[lint::skip(simpler_numeric_expression)]
    public fun test8(x: u8): u64 {
        0 >> x + 0 << x
    }
}
