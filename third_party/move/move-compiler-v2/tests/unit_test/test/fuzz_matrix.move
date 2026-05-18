// Matrix expansion: `a = [...]` produces one test case per element.
module 0x1::M {
    #[test(_a = [@0x1, @0x2, @0x3])]
    public fun matrix_single(_a: signer) { }

    #[test(_a = [@0x1, @0x2], _b = [@0xa, @0xb])]
    public fun matrix_cartesian(_a: signer, _b: signer) { }

    // Singleton matrix [v] behaves like `a = v`.
    #[test(_a = [@0x1])]
    public fun matrix_singleton(_a: signer) { }
}
