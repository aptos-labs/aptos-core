module 0x42::A {
    fun foo() {}
    public(package) fun bar() {}
}

module 0x43::B {
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
