module 0x42::A {
    public(package) fun foo() {
        0x42::B::foo()
    }
}

module 0x42::B {
    public(package) fun foo() {
        0x42::A::foo()
    }
}
