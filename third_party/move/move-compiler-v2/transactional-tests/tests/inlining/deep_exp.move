//# publish
module 0x42::Test {

    inline fun f1(x: u64): u64 {
        f2(f2(f2(f2(f2(x)))))
    }

    inline fun f2(x: u64): u64 {
        f3(f3(f3(f3(f3(x)))))
    }

    inline fun f3(x: u64): u64 {
        f4(f4(f4(f4(f4(x)))))
    }

    inline fun f4(x: u64): u64 {
        f5(f5(f5(f5(f5(x)))))
    }

    inline fun f5(x: u64): u64 {
        x + 1
    }

    public fun test(): u64 {
        f1(0)
    }
}

//# run 0x42::Test::test
