//# publish
module 0xc0ffee::m {
    public fun test(x: u64, y: u64): (u64, u64) {
        let t = x;
        x = y;
        y = t;
        (x, y)
    }

    public fun main() {
        let (a, b) = test(55, 66);
        assert!(a == 66, 0);
        assert!(b == 55, 0);
    }

}

//# run 0xc0ffee::m::main
