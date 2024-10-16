//# publish
module 0xc0ffee::m {
    fun id(x: u64): u64 {
        x
    }

    public fun test(): u64 {
        let _x = id(42);
        _x = id(44);
        _x = id(46);
        let y = _x + 1;
        y
    }
}

//# run 0xc0ffee::m::test
