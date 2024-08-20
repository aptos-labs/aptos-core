//# publish --print-bytecode
module 0x42::Test {

    inline fun f1(x: u64): u64 {
        add1(sub1(add1(sub1(add1(sub1(add1(sub1(add1(sub1(add1(sub1(x))))))))))))
    }

    inline fun f2(x: u64): u64 {
        f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(f1(x))))))))))))))))
    }

    inline fun f3(x: u64): u64 {
        f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(f2(x))))))))))))))))
    }

    inline fun add1(x: u64): u64 {
        x + 1
    }

    inline fun sub1(x: u64): u64 {
        x - 1
    }

    public fun test(x: u64): u64 {
        f3(x)
    }
}

//# run 0x42::Test::test --args 10241024
