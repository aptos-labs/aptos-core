//# publish
module 0xc0ffee::m {
    struct S has drop {
        x: u64,
        y: u64,
        z: u64,
    }

    public fun test(): u64 {
        let x = 1;
        let s = S { x, y: {x = x + 1; x}, z: {x = x + 1; x} };
        let S {x, y, z} = s;
        x + y + z
    }
}

//# run 0xc0ffee::m::test
