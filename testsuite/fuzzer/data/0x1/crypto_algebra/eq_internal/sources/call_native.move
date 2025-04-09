module poc::eq_internal {
    use aptos_std::crypto_algebra::{Self};
    use aptos_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let five_1 = crypto_algebra::from_u64<Fr>(5);
        let five_2 = crypto_algebra::from_u64<Fr>(5);
        let six = crypto_algebra::from_u64<Fr>(6);

        assert!(crypto_algebra::eq(&five_1, &five_2), 0);

        assert!(!crypto_algebra::eq(&five_1, &six), 1);
        assert!(!crypto_algebra::eq(&six, &five_1), 2);

        assert!(crypto_algebra::eq(&five_1, &five_1), 3);
        assert!(crypto_algebra::eq(&six, &six), 4);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
