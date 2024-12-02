//# publish
module 0x8675::M {
    inline fun test1(r: u64): u64 {
        let t = r;       // t = 10
        let t2 = 0;      // t2 = 0
        let f = |x| { if (x) { 2 } else { false } };
        while (r > 0) {
            let x = r;   // x = 10,  9,  8,  7,  6,  5,  4,  3,  2,  1
            r = r - 1;   // r =  9,  8,  7,  6,  5,  4,  3,  2,  1,  0
            t2 = t2 + x; // t2= 10, 19, 27, 34, 40, 45, 49, 52, 54, 55
        };
        let t3 = r + t + t2; // 0 + 10 + 55 = 65
        t3 // 65
    }
    public fun test(): u64 {
        test1(10)
    }
}

//# run 0x8675::M::test
