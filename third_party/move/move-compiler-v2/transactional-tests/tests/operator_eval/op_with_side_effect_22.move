//# publish
module 0xc0ffee::m {
    fun add3(x: u64, y: u64, z: u64): u64 {
        x + y + z
    }

    public fun test(): u64 {
        add3(abort 0, {abort 14; 0}, 0)
    }
}

//# run 0xc0ffee::m::test
