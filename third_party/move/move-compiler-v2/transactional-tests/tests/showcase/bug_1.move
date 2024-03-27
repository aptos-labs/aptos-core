//# publish
module 0xc0ffee::m {
    fun five(): u64 {
        5
    }

    fun six(): u64 {
        6
    }

    fun test(): u64 {
        let _t = five();
        _t = six();
        _t
    }
}

//# run 0xc0ffee::m::test
