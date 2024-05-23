//# publish
module 0xc0ffee::m {
    public fun test(): u8 {
        {250u8 + 50u8} + {return 55; 5u8}
    }
}

//# run 0xc0ffee::m::test
