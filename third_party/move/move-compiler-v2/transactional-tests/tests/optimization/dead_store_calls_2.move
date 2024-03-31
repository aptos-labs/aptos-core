//# publish
module 0xc0ffee::m {
    fun four(): u64 {
        4
    }

    fun ten(): u64 {
        10
    }

    public fun test(): u64 {
        let _x = four();
        _x = ten();
        _x
    }
}

//# run 0xc0ffee::m::test
