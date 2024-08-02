//# publish
module 0xc0ffee::m {
    public fun test(p: u64): u64 {
        let x = p;
        (x + 1) + {x = x + 1; x + 1} + {x = x + 1; x + 1}
    }
}

//# run 0xc0ffee::m::test --args 54
