// It's ok to use (public)friend and public(package) in different modules

module 0x42::A {
    public(friend) fun foo() {
        0x42::B::foo();
    }
}

module 0x42::B {
    public(package) fun foo() {

    }
}
