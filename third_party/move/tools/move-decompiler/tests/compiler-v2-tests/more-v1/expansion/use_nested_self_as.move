module 0x2::X {
    struct S {
    }
    public fun foo() {
        ()
    }
}
module 0x2::M {
    use 0x2::X;
    struct X {
        f: 0x2::X::S,
        f2: 0x2::X::S,
    }
    fun bar() {
        X::foo();
        X::foo();
    }
}
