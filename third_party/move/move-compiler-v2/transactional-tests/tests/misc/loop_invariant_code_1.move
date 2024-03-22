//# publish
module 0xc0ffee::m {

    public fun test(p: u64): u64 {
        let a = p;
        let count = 0;
        while (count < 10) {
            a = p;
            count = count + 1;
        };
        a // copy `a := p` should be available
    }
}

//# run 0xc0ffee::m::test --args 44

//# run 0xc0ffee::m::test --args 6
