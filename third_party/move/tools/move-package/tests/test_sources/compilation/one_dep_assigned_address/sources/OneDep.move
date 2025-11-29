module A::OneDep {
    use B::B;

    public fun do_b() {
        B::foo()
    }
}
