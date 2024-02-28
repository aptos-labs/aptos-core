//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let t = 1;
        t = 2;
        t
    }
}

//# run 0xc0ffee::m::test
