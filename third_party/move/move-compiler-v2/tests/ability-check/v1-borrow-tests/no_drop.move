module 0x8675309::M {
    struct X {}

    fun t1(): bool {
        let x = X {};
        &x;
        false
    }
}
