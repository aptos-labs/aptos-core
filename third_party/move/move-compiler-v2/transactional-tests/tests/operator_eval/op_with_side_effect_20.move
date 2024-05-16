//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 1;
        {x = x + 1; x = x + 1; x } + {x = x + 1; x = x + 1; x } + {x = x + 1; x = x + 1; x}
    }

}

//# run 0xc0ffee::m::test
