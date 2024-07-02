module 0x42::A {
    // check we don't add duplicate `friend 0x42::B;`
    // during the transformation
    friend 0x42::B;
    fun foo() {}
    public(package) fun bar() {}
}

module 0x42::B {
    use 0x42::A;

    public(package) fun foo() {
        A::bar()
    }

    public fun bar() {
        A::bar()
    }

    fun baz() {
        A::bar()
    }
}

module 0x42::C {
    public(package) fun foo() {
        bar()
    }

    public(package) fun bar() {

    }
}
