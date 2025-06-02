//# publish
module 0x77::m {
    struct Func(|&u64|bool) has drop;

    fun test() {
        let f: Func = |x| *x > 0;
        assert!(f(&1))
    }
}

//# run 0x77::m::test
