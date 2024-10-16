//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let a = 1;
        let x;
        let y;
        let z;
        (x, y, z) = (a, {a = a + 1; a}, {a = a + 1; a});
        x + y + z
    }
}

//# run 0xc0ffee::m::test
