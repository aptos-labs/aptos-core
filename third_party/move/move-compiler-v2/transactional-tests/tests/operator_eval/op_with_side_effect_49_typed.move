//# publish
module 0xc0ffee::m {
    inline fun call(f: |u64|u64): u64 {
        f(2)
    }

    public fun test(): u64 {
        let x = 1;
        x + call(|_x: u64| {x = x + 1; x}) + call(|_x: u64| {x = x + 7; x})
    }
}

//# run 0xc0ffee::m::test
