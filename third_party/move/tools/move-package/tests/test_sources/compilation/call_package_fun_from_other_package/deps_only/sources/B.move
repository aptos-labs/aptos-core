module B::B {
    use B::C;

    public(package) fun foo() {
        C::bar()
    }
}

module B::C {
    public(package) fun bar() {

    }
}
