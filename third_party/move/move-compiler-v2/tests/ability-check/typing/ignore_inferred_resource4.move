module 0x8675309::M {
    struct S<phantom T> has drop {}
    fun no<T: drop>() {
        S<T>{};
    }
}
