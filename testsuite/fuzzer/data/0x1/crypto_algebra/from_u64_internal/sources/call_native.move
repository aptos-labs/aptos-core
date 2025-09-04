module poc::from_u64_internal {
    use velor_std::crypto_algebra::{Self};
    use velor_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let val1_u64: u64 = 12345;
        let val2_u64: u64 = 67890;

        let elem1_a = crypto_algebra::from_u64<Fr>(val1_u64);
        let elem1_b = crypto_algebra::from_u64<Fr>(val1_u64);
        let elem2 = crypto_algebra::from_u64<Fr>(val2_u64);

        assert!(crypto_algebra::eq(&elem1_a, &elem1_b), 0);

        assert!(!crypto_algebra::eq(&elem1_a, &elem2), 1);
        assert!(!crypto_algebra::eq(&elem1_b, &elem2), 2);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
