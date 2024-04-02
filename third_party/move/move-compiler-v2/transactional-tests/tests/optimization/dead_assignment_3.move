//# publish
module 0xc0ffee::m {
    public fun test(p: bool): u32 {
        let x = 1;
        let _y = x;
        if (p) {
            _y = _y;
            _y = _y;
            _y
        } else {
            _y = _y;
            9
        }
    }
}

//# run 0xc0ffee::m::test --args true

//# run 0xc0ffee::m::test --args false
