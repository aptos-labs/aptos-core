module B::B {
    public(package) fun foo() {
        0x43::B::bar()
    }
}

module 0x43::B {
    public(package) fun bar() {

    }
}

module B::C {
    public fun baz() {
        B::B::foo()
    }
}
