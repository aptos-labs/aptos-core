//# publish
module 0xc0ffee::m {
    public fun aborter(x: u64): u64 {
        abort x
    }

    public fun test(): u64 {
        let x = 1;
        aborter(x) + {x = x + 1; aborter(x + 100); x} + x
    }
}

//# run 0xc0ffee::m::test
