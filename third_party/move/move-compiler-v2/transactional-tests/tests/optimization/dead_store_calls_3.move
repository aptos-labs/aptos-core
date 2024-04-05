//# publish
module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    public fun test(p: u64): u64 {
        let _x = one();
        _x = p;
        _x
    }
}

//# run 0xc0ffee::m::test --args 42
