//# publish
module 0xc0ffee::m {
    fun add2(x: u64, y: u64): u64 {
        x + y
    }

    fun add3(x: u64, y: u64, z: u64): u64 {
        x + y + z
    }

    public fun test(): u64 {
        let x = 1;
        add3(x, {x = add2(x, 1); x}, {x = add2(x, 1); x})
    }
}

//# run 0xc0ffee::m::test
