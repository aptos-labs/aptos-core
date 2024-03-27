//# publish
module 0xc0ffee::m {

    fun test(): u64 {
        let _t = 1;
        _t = 5;
        _t
    }
}

//# run 0xc0ffee::m::test
