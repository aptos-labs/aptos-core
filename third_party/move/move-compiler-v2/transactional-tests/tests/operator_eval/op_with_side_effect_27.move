//# publish
module 0xc0ffee::m {
    public fun test(p: bool): bool {
        (!p && {p = p && false; p}) || {p = !p; !p}
    }
}

//# run 0xc0ffee::m::test --args true

//# run 0xc0ffee::m::test --args false
