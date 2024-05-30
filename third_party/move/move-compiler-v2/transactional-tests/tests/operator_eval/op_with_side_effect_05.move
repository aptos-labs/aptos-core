//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let _x = 1;
        {let y = _x + 2; _x = _x + 5; y} + {let y = _x; _x = _x - 1; y} + {let y = _x; _x = _x * 2; y}
    }

}

//# run 0xc0ffee::m::test
