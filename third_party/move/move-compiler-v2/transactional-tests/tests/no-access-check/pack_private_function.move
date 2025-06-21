//# publish
module 0xc0ffee::m {
    struct S(u64);

    fun inaccessible() {}
}

//# publish
module 0xc0ffee::n {
    public fun test() {
        let f = || 0xc0ffee::m::inaccessible();
        f();
    }
}
