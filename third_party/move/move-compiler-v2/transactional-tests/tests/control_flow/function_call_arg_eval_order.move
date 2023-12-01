//# publish
module 0xc0ffee::m {
    fun foo(x: u64, y: u64): u64 {
        y - x
    }

    public fun test1(): u64 {
        let a = 1;
        foo(a, {a = a + 1; a})
    }

    fun bar(x: &u64, y: &u64): u64 {
        *y - *x
    }

    public fun test2(): u64 {
        let a = 1;
        let b = 10;
        let c = &a;
        bar(c, {c = &b; c})
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2
