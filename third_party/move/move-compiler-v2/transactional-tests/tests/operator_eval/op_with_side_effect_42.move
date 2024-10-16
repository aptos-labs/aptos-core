//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 1;
        let (a, b, c) = (x + 1, {x = x + 1; x + 7}, {x = x + 1; x - 3});
        a + b + c
    }
}

//# run 0xc0ffee::m::test
