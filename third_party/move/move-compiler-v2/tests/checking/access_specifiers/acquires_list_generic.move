module 0x42::M {
    struct CupC<phantom T> {}
    struct R {}
    struct B<phantom T> {}

    fun foo() acquires B<CupC<R>> {
        abort 0
    }
}
