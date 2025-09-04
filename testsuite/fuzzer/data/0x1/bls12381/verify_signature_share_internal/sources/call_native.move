module poc::verify_signature_share_internal {
    use velor_std::bls12381::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let pk_bytes = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7";
        let pop_bytes = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364";
        let message = b"message share";
        let invalid_sig_share_bytes = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let pop = bls12381::proof_of_possession_from_bytes(pop_bytes);
        let maybe_pk_with_pop = bls12381::public_key_from_bytes_with_pop(pk_bytes, &pop);
        assert!(option::is_some(&maybe_pk_with_pop), 1);
        let pk_with_pop = option::extract(&mut maybe_pk_with_pop);

        let invalid_sig_share = bls12381::signature_from_bytes(invalid_sig_share_bytes);

        let result_fail = bls12381::verify_signature_share(&invalid_sig_share, &pk_with_pop, message);
        assert!(!result_fail, 101);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
