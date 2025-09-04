module poc::downcast_internal {
    use std::option::{Self};
    use velor_std::crypto_algebra::{Self};
    use velor_std::bls12381_algebra::{Fq12, Gt};

    public entry fun main(_owner: &signer) {
        let zero_gt = crypto_algebra::zero<Gt>();
        let zero_fq12_up = crypto_algebra::upcast<Gt, Fq12>(&zero_gt);

        let downcast_result_ok_opt = crypto_algebra::downcast<Fq12, Gt>(&zero_fq12_up);
        assert!(option::is_some(&downcast_result_ok_opt), 0);
        let downcast_result_ok = option::extract(&mut downcast_result_ok_opt);
        assert!(crypto_algebra::eq(&downcast_result_ok, &zero_gt), 1);

        let two_fq12 = crypto_algebra::from_u64<Fq12>(2);
        let downcast_result_fail_opt = crypto_algebra::downcast<Fq12, Gt>(&two_fq12);
        assert!(option::is_none(&downcast_result_fail_opt), 2);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
