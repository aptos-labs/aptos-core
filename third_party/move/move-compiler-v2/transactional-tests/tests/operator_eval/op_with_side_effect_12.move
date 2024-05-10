//# publish
module 0xc0ffee::m {
    public fun aborter(x: u64): u64 {
        abort x
    }

    public fun test(p: bool): u64 {
        if (p) {aborter(1)} else {abort 2} + {abort 3; 0}
    }
}

//# run 0xc0ffee::m::test --args true

//# run 0xc0ffee::m::test --args false
