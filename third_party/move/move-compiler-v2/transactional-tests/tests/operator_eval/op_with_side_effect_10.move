//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 122;
        {x = x - 1; x + 8} + {x = x + 3; x - 3} + {x = x * 2; x * 2}
    }
}

//# run 0xc0ffee::m::test
