//# publish
module 0xc0ffee::m {
    public fun test(p: bool): u64 {
        let x = 1;
        if (p) {x} else {x = x + 1; x} + if (!p) {x} else {x = x + 1; x} + if (!p) {x} else {x = x + 1; x}
    }
}

//# run 0xc0ffee::m::test --args true

//# run 0xc0ffee::m::test --args false
