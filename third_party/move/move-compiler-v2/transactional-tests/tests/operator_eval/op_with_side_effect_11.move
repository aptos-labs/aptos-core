//# publish
module 0xc0ffee::m {
    public fun aborter(x: u64): u64 {
        abort x
    }

    public fun test(p: bool, x: u64): u64 {
        if (p) {aborter(x)} else {aborter(x+100)} + {aborter(x+200); x}
    }

}

//# run 0xc0ffee::m::test --args true 1
