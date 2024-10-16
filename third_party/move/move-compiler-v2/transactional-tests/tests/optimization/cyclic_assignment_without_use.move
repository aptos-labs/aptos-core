//# publish
module 0xc0ffee::m {
    public fun test(x: u64) {
        let _a = x;
        let b = _a;
        let c = b;
        _a = c;
    }

}

//# run 0xc0ffee::m::test --args 55
