//# publish
module 0xc0ffee::m {
    fun add3(x: u64, y: u64, z: u64): u64 {
        x + y + z
    }

    public fun test1(): u64 {
        let x = 1;
        x + add3(x, {x = inc(&mut x); add3(x, {x = x + 1; x}, {x = x + 1; x})}, {x = inc(&mut x); add3({x = x + 1; x}, x, {x = x + 1; x})}) + {x = inc(&mut x) + 1; x}
    }

    fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    fun inc_by(x: &mut u64, y: u64): u64 {
        *x = *x + y;
        *x
    }

    public fun test2(): u64 {
        let x = 1;
        x + add3(x, {x = inc_by(&mut x, 3); add3(x, {x = x + 1; x}, {x = x + 1; x})}, {x = inc(&mut x); add3({x = x + 1; x}, x, {x = x + 1; x})}) + {x = inc_by(&mut x, 47) + 1; x}
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2
