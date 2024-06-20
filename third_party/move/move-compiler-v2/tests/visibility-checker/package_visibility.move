module 0x42::A {
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
    use 0x42::B;

    public(package) fun foo() {
        B::foo()
    }

    public fun bar() {
        B::foo()
    }

    fun baz() {
        B::foo()
    }
}
