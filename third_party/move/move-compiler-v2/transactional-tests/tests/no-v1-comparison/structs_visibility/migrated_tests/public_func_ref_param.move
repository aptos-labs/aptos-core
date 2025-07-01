//# publish
module 0x77::m {
    public struct Func(|&u64|bool) has drop;
}

//# publish
module 0x77::m_func_ref_param {
    use 0x77::m::Func;

    fun test() {
        let f: Func = |x| *x > 0;
        assert!(f(&1))
    }
}

//# run 0x77::m_func_ref_param::test
