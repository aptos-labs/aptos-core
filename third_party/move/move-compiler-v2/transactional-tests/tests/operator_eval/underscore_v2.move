//# publish
module 0xc0ffee::m {
    fun test2(): (u32, u64) {
        (1u32, 2u64)
    }

    // public fun test(_: u64): u64 {
    //     let x = _ + 3;
    //     x + _
    // }

    // public fun test3(_: u64): u64 {
    //     let (_, _) = test2();
    //     let x = _ + 3;
    //     _ = _ + 1;
    //     x + _
    // }

    // public fun test4(_: u64): u64 {
    //     let (_, _x) = test2();
    //     _ = _ + 2;
    //     _
    // }

    // public fun test5(_: u64): u64 {
    //     let (_, _) = test2();
    //     _ = _ + 3;
    //     _
    // }

    // public fun test6(_: u64): u64 {
    //     let (_x, _) = test2();
    //      _
    // }

    // public fun test7(_y: u64): u64 {
    //     let _ = _y;
    //     _  // undefined
    // }

    // public fun test8(_: u64, _: u64): u64 {
    //     let _ = _;
    //     _
    // }

    inline fun fun9(x: u64, _: u64, _: u64): u64 {
        x + 3
    }

    public fun test9(): u64 {
        fun9(4, 3, 2)
    }

    inline fun fun10(x: u64, _: u64, _: |u64|u64): u64 {
        x + 3
    }

    // public fun test10a(): u64 {
    //     fun10(4, 3, 2)
    // }

    public fun test10b(): u64 {
        fun10(4, 3, |x|x + 1)
    }

    public fun test10c(): u64 {
        fun10(4, 3, |_|1)
    }

    inline fun fun11(x: u64, _: u64, f: |u64|u64): u64 {
        f(x)
    }

    public fun test11(): u64 {
        fun11(4, 3, |_|1)
    }
}

// //# run 0xc0ffee::m::test --args 4

// //# run 0xc0ffee::m::test3 --args 5

// //# run 0xc0ffee::m::test4 --args 5

// //# run 0xc0ffee::m::test5 --args 5

// //# run 0xc0ffee::m::test6 --args 5

// //# run 0xc0ffee::m::test7 --args 5

// //# run 0xc0ffee::m::test8 --args 5

//# run 0xc0ffee::m::test9

// //# run 0xc0ffee::m::test10a

//# run 0xc0ffee::m::test10b

//# run 0xc0ffee::m::test10c

//# run 0xc0ffee::m::test11
