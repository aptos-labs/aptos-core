//# publish
module 0xc0ffee::m {
    public fun test(): u8 {
        abort (1u64 - 10u64)
    }
}

//# run 0xc0ffee::m::test
