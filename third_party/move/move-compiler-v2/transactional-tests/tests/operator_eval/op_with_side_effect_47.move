//# publish
module 0xc0ffee::m {
    inline fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun test(): u64 {
        let x = 1;
        x + inc(&mut x) + inc(&mut x)
    }
}

//# run 0xc0ffee::m::test
