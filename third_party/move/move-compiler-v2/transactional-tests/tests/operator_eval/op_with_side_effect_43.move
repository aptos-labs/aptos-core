//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 1;
        (x + {x = x + 1; x - 1}) + {x = x + 1; x * 2}
    }
}

//# run 0xc0ffee::m::test
