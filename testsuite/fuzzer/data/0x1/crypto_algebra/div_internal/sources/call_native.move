module poc::div_internal {
    use std::option::{Self};
    use aptos_std::crypto_algebra::{Self};
    use aptos_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let x = crypto_algebra::from_u64<Fr>(6);
        let y = crypto_algebra::from_u64<Fr>(2);
        let z_expected = crypto_algebra::from_u64<Fr>(3);
        let zero = crypto_algebra::zero<Fr>();

        let result_option = crypto_algebra::div(&x, &y);
        assert!(option::is_some(&result_option), 0);
        let z_actual = option::extract(&mut result_option);
        assert!(crypto_algebra::eq(&z_actual, &z_expected), 1);

        let result_div_zero = crypto_algebra::div(&x, &zero);
        assert!(option::is_none(&result_div_zero), 2);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
