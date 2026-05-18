//# publish
module 0xc0ffee::m {
    public fun test(p: bool, q: i64): i64 {
        if (p) return -56 + q;
        -88
    }
}

//# run 0xc0ffee::m::test --args true 100
