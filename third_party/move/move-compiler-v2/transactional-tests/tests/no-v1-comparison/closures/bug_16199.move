//# publish
module 0xc0ffee::m {
    public fun test() {
        let f = |x| |y| |z| x + y + z;
        assert!(f(1)(2)(3) == 6);
    }
}
//# run 0xc0ffee::m::test
