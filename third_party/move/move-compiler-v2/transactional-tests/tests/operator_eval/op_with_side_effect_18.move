//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 1;
        {let one = 1; x = x + one; x} + {{let two = 2; x = x + two; x} + {let three = 3; x = x + three; x} + x} + {x = x + 1; x = x + 1; x }
    }
}

//# run 0xc0ffee::m::test
