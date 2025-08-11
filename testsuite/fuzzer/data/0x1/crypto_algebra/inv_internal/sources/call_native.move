module poc::inv_internal {
    use std::option::{Self};
    use aptos_std::crypto_algebra::{Self};
    use aptos_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let one = crypto_algebra::one<Fr>();
        let zero = crypto_algebra::zero<Fr>();
        let x = crypto_algebra::from_u64<Fr>(5);

        let x_inv_opt = crypto_algebra::inv(&x);
        assert!(option::is_some(&x_inv_opt), 0);

        let x_inv = option::extract(&mut x_inv_opt);

        let product = crypto_algebra::mul(&x, &x_inv);
        assert!(crypto_algebra::eq(&product, &one), 1);

        let zero_inv_opt = crypto_algebra::inv(&zero);
        assert!(option::is_none(&zero_inv_opt), 2);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
