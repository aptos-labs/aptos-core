module poc::verify_proof_of_possession_internal {
    use aptos_std::bls12381::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let pk_bytes = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7";
        let pop_bytes_valid = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364";
        let pop_bytes_invalid = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let pop_valid = bls12381::proof_of_possession_from_bytes(pop_bytes_valid);
        let pop_invalid = bls12381::proof_of_possession_from_bytes(pop_bytes_invalid);

        let result_ok = bls12381::public_key_from_bytes_with_pop(pk_bytes, &pop_valid);
        assert!(option::is_some(&result_ok), 1);

        let result_fail = bls12381::public_key_from_bytes_with_pop(pk_bytes, &pop_invalid);
        assert!(option::is_none(&result_fail), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
