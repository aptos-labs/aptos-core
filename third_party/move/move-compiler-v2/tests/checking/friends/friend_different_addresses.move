module 0xc0ffee::m {
    friend 0xdeadbeef::n;
    public fun test() {}
}

module 0xdeadbeef::n {
    public fun test() {}
}
