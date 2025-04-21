module 0x815::a {
    package fun f() {}
    public(package) fun g(){}
}

module 0x815::b {
    friend 0x815::c;
    friend fun f() {}
}

module 0x815::c {
    public fun f() {
        0x815::a::f();
        0x815::a::g();
        0x815::b::f();
    }
}
