//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let a = 1;
        let (x, y): (&u64, &u64) = (&mut a, freeze(&mut a)); // bug in v1, works in v2
        *x + *y
    }
}

//# run 0xc0ffee::m::test
