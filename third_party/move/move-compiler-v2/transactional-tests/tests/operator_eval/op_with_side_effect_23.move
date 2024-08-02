//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        (abort 0) + {(abort 14); 0} + 0
    }
}

//# run 0xc0ffee::m::test
