module supra_std::bls12381_scalar {

    use std::error;
    use std::features;
    use std::option::Option;
    use aptos_std::bls12381_algebra::{Fr, FormatFrLsb};
    use aptos_std::crypto_algebra::{deserialize, Element};
    #[test_only]
    use std::option;
    #[test_only]
    use aptos_std::crypto_algebra::{eq, zero};

    /// The native functions have not been rolled out yet.
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 1;

    public fun bls12381_hash_to_scalar(
        dst: vector<u8>,
        msg: vector<u8>,
    ): Option<Element<Fr>> {
        assert!(features::supra_private_poll_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));
        let scalar_bytes = native_hash_to_scalar(dst, msg);
        deserialize<Fr, FormatFrLsb>(&scalar_bytes)
    }

    native fun native_hash_to_scalar(
        dst: vector<u8>,
        msg: vector<u8>,
    ): vector<u8>;

    #[test(fx = @supra_framework)]
    public fun test_hash_to_scalar(fx: signer) {

        features::change_feature_flags_for_testing(&fx, vector[ features::get_supra_private_poll_feature() ], vector[]);

        let msg: vector<u8> = b"1234";
        let dst: vector<u8> = b"5678";

        let scalar = bls12381_hash_to_scalar(msg, dst);
        assert!(!eq<Fr>(&option::extract(&mut scalar), &zero<Fr>()) , 1);
    }

}
