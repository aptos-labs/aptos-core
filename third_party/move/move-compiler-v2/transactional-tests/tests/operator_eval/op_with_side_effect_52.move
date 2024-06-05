//# publish
module 0xc0ffee::m {
    public fun test(p: u64): u64 {
        1 + (p + {p = p + 1; p})
    }
}

//# run 0xc0ffee::m::test --args 1
