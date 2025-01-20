//# publish
module 0xc0ffee::m {
    fun foo1(a: u64, b: u64, c: bool): u64 {
        if (c) {
            a = b;
        };
        let t = a * 2;
        bar();
        let t2 = a * 3;
        t + t2
    }

    fun foo2(a: u64, b: u64, c: bool): u64 {
        if (c) {
            a = b;
        };
        let t = a * 2;
        if (c) {
            bar();
        };
        bar();
        let t2 = a * 3;
        t + t2
    }

    fun bar() {
        assert!(true, 1);
    }
}

//# run 0xc0ffee::m::foo1 --args 10 20 true

//# run 0xc0ffee::m::foo1 --args 10 20 false

//# run 0xc0ffee::m::foo2 --args 10 20 true

//# run 0xc0ffee::m::foo2 --args 10 20 false
