//# publish
module 0xc0ffee::m {
    public fun test(x: u64, y: u64): (u64, u64) {
        let i = x;
        let t;
        while (i > 0) {
            t = x;
            x = y;
            y = t;
            i = i - 1;
        };
        (x, y)
    }

    public fun main() {
        let (a, b) = test(9, 55);
        assert!(a == 55, 0);
        assert!(b == 9, 0);
    }
}

//# run 0xc0ffee::m::main
