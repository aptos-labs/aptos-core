module poc::mul_internal {
    use aptos_std::crypto_algebra::{Self};
    use aptos_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let zero = crypto_algebra::zero<Fr>();
        let one = crypto_algebra::one<Fr>();
        let two = crypto_algebra::from_u64<Fr>(2);
        let three = crypto_algebra::from_u64<Fr>(3);
        let six = crypto_algebra::from_u64<Fr>(6);

        let product = crypto_algebra::mul(&two, &three);
        assert!(crypto_algebra::eq(&product, &six), 0);

        let product_comm = crypto_algebra::mul(&three, &two);
        assert!(crypto_algebra::eq(&product_comm, &six), 1);

        let product_zero = crypto_algebra::mul(&two, &zero);
        assert!(crypto_algebra::eq(&product_zero, &zero), 2);

        let product_one = crypto_algebra::mul(&one, &six);
        assert!(crypto_algebra::eq(&product_one, &six), 3);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
