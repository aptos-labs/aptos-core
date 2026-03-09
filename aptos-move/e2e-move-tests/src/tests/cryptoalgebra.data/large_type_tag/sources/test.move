module 0x42::test {
    use aptos_std::bls12381_algebra::G1;
    
    use aptos_std::crypto_algebra;

    struct Foo<
        phantom A,
        phantom B,
        phantom C,
        phantom D,
    >{}

    /// Type to tag conversion failed because type tag budget is exceeded.
    public entry fun main() {
        crypto_algebra::hash_to<
            G1,
            Foo<
                Foo<Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>>,
                Foo<Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>>,
                Foo<Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>>,
                Foo<Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>, Foo<u8, u8, u8, u8>>,
            >
        >(&b"hello", &b"world");
    }
}
