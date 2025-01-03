//# publish
module 0xc0ffee::m {
    fun call(f: ||u64): u64 {
        f()
    }

    public fun test(): u64 {
        let x = 1;
        x + call(|| {x = x + 1; x}) + call(|| {x = x + 7; x})
    }
}

//# run 0xc0ffee::m::test
