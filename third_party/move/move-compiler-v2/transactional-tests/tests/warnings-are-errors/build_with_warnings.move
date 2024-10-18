//# publish --print-bytecode
module 0x42::m {
    public fun foo(x: u64): u64 {
        1 + 1
    }

    public fun bar() {
        assert!(foo(3) == 2, 1);
    }
}

//# run 0x42::m::bar
