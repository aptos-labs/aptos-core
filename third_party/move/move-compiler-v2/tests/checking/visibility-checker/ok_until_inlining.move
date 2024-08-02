module 0xc0ffee::m {
    public inline fun foo() {
        priv();
    }

    fun priv() {}

    fun test() {
        foo(); // ok
    }
}

module 0xc0ffee::n {
    fun test() {
        0xc0ffee::m::foo(); // not ok
    }
}
