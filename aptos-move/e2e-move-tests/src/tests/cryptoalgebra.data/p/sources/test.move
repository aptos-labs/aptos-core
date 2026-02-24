module 0xbeef::test {
    use aptos_std::crypto_algebra;
    use aptos_std::bls12381_algebra::G1;

    // A phantom struct with 8 type parameters (valid type tag).
    struct W<
        phantom T1, phantom T2, phantom T3, phantom T4,
        phantom T5, phantom T6, phantom T7, phantom T8
    > {}

    /// Creates a type that exceeded type tag budget during type to tag conversion.
    public entry fun run() {
        crypto_algebra::hash_to<
            G1,
            W<
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>,
                W<u8, u8, u8, u8, u8, u8, u8, u8>
            >
        >(&b"DST", &b"message");
    }
}
