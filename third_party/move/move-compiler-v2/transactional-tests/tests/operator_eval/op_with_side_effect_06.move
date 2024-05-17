//# publish
module 0xc0ffee::m {
    fun inc(x: &mut u64, y: u64): u64 {
        *x = *x + y;
        *x
    }

    public fun test(): u64 {
        let x = 1;
        inc(&mut x, 5) + inc(&mut x, 6) + inc(&mut x, 7)
    }

}

//# run 0xc0ffee::m::test
