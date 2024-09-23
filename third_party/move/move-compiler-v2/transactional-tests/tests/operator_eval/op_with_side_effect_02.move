//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 10;
        let y = 20;
        {let (x, y) = (y, x + 1); x + y} + {let (x, y) = (y * 2, x - 1); y / x}
    }
}

//# run 0xc0ffee::m::test
