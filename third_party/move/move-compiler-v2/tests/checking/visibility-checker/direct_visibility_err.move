module 0x815::b {
    friend fun f() {}
}

module 0x815::c {
    public fun f() {
        0x815::b::f();
    }
}
