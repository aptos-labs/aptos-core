module aptos_std::M {
    #[a, a(x = 0)]
    fun foo() {}

    #[testonly]
    #[b(a, a = 0, a(x = 1))]
    fun bar() {}
}
