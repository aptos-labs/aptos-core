//# publish
module 0xc0ffee::m {
    fun inc(x: &mut u64, by: u64): u64 {
        *x = *x + by;
        *x
    }

    public fun test(): u64 {
        let x = 1;
        x + inc(&mut x, 7) + inc(&mut x, 11)
    }
}

//# run 0xc0ffee::m::test
