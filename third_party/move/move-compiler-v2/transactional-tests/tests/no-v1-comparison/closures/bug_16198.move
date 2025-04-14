//# publish
module 0xc0ffee::m {
    struct Func(|| | |u64) has copy, drop;
    fun forty_two(): u64 {
        42
    }

    public fun test(x: u64) {
        let f: | | | | u64 has drop = || || x;
        assert!(f()() == 42);
        let y = forty_two();
        let f2: | | | | u64 has drop = || || y;
        assert!(f2()() == 42);
        let f3: | | | | | | u64 has drop = || || || x;
        assert!(f3()()() == 42);
        let f4: | | | | | | | | u64 has drop = || {
            let z = || x;
            let a = z();
            || || || a + y
        };
        assert!(f4()()()() == 84);
        let f5: | | | | | | | | u64 has drop = || {
            let z = || y;
            let a = z();
            || || || a + x
        };
        assert!(f5()()()() == 84);
        let f = Func(|| ||x);
        assert!(f()() == 42);
    }
}
//# run 0xc0ffee::m::test --args 42
