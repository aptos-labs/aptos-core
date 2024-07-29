//# publish
module 0xc0ffee::m {
    inline fun inc(x: u64): u64 {
        x = x + 1;
        x
    }

    public fun test(): u64 {
        let x = 1;
        x + inc(x) + inc(x)
    }
}

//# run 0xc0ffee::m::test
