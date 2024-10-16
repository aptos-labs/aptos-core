//# publish
module 0xc0ffee::m {
    public fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    public fun test(): u64 {
        let x = 1;
        {x = inc(&mut x) + 1; x} + x + {x = inc(&mut x) + 1; x}
    }
}

//# run 0xc0ffee::m::test
