//# publish
module 0xc0ffee::m {
    public fun test1(x: u64, a: u64, b: u64): u64 {
        let i = 0;
        while (i < x) {
            a = b;
            b = a;
            i = i + 1;
        };
        a
    }

    public fun test2(x: u64, a: u64): u64 {
        let i = 0;
        while (i < x) {
            a = a;
            i = i + 1;
        };
        a
    }

}

//# run 0xc0ffee::m::test1 --args 0 55 66

//# run 0xc0ffee::m::test1 --args 5 55 66

//# run 0xc0ffee::m::test2 --args 0 55

//# run 0xc0ffee::m::test2 --args 5 55
