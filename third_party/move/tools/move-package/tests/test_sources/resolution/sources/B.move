module B::M {
    struct S has drop {}
    fun foo(_: S) {}
    fun bar() {
        foo(A::M::S {})
    }
}
