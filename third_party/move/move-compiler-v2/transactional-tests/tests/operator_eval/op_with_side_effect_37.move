//# publish
module 0xc0ffee::m {
    fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    struct S has drop {
        x: u64,
        y: u64,
        z: u64,
    }

    public fun test(): u64 {
        let x = 1;
        let s = S { x, y: inc(&mut x), z: inc(&mut x) };
        let x;
        let y;
        let z;
        S {x, y, z} = s;
        x + y + z
    }
}

//# run 0xc0ffee::m::test
