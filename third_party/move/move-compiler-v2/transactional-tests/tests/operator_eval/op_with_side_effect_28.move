//# publish
module 0xc0ffee::m {
    public fun test(): bool {
        let x = 1;
        {x = x << 1; x} < {x = x << 1; x}
    }
}

//# run 0xc0ffee::m::test
