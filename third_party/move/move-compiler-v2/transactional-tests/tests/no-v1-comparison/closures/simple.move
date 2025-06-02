//# publish
module 0xc0ffee::m {
    fun foo(a: u64, b: u64, c: u64, d: u64): u64 {
        a + b + c + d
    }

    fun one(): u64 {
        1
    }

    public fun test(): u64 {
        let _ = one();
        let _ = one();
        let _ = one();

        let f = |y| 0xc0ffee::m::foo(1, 1, y, y);
        f(1)
    }
}

//# run 0xc0ffee::m::test
