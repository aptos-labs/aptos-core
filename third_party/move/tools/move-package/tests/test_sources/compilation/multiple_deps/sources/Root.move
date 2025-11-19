module A::A {
    use D::A as DA;
    use C::A as CA;

    public fun foo() {
        DA::foo();
        CA::foo()
    }
}
