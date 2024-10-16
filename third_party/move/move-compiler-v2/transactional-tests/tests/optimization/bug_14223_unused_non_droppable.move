//# publish
module 0xCAFE::m {
    struct S {}

    public fun f() {
        let _x = S {};
        abort 1;
        let S {} = _x;
    }
}

//# run 0xCAFE::m::f
