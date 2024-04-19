//# publish
module 0xc0ffee::m {
    fun foo(): (u64, u64) {
        let a = 1;
        (a, {a = a + 1; a})
    }

    public fun test(): u64 {
        let (a, b) = foo();
        a + b
    }
}

//# run 0xc0ffee::m::test
