//# publish
module 0xc0ffee::m {
    fun inc(x: &mut u64, by: u64): u64 {
        *x = *x + by;
        *x
    }

    struct S has drop {
        x: u64,
        y: u64,
        z: u64,
    }

    public fun test(): u64 {
        let x = 1;
        let S {x, y, z} = S { x, y: inc(&mut x, 7), z: inc(&mut x, 11) };
        x + y + z
    }
}

//# run 0xc0ffee::m::test
